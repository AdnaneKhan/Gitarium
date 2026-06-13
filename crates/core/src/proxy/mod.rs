//! Optional GitHub API proxy. When the server enables it (`serve.ts
//! --api-proxy`) the browser stops calling api.github.com directly: each
//! GitHub request is forwarded over a WebSocket to the server, which performs
//! the fetch and forwards the response back. Disabled by default — every call
//! falls straight through to `fetch::request`, exactly as before. AI/Anthropic
//! traffic never comes through here. Hard-fail: when enabled, a downed socket
//! errors the request (it never silently hits GitHub directly) and reconnects
//! lazily on the next call.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use serde_json::Value;

use crate::fetch::{self, HttpResponse};

mod socket;

const API: &str = "https://api.github.com";

type Shared<T> = Rc<RefCell<WaiterState<T>>>;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum State {
    Disconnected,
    Connecting,
    Open,
}

thread_local! {
    static CONFIG: RefCell<Option<String>> = RefCell::new(None);
    static NEXT_ID: Cell<u64> = Cell::new(1);
    pub(crate) static STATE: Cell<State> = Cell::new(State::Disconnected);
    pub(crate) static PENDING: RefCell<HashMap<u64, Shared<Result<HttpResponse, String>>>> =
        RefCell::new(HashMap::new());
    pub(crate) static OPEN_WAITERS: RefCell<Vec<Shared<Result<(), String>>>> =
        RefCell::new(Vec::new());
}

/// Configure proxying. `url` is the resolved ws(s):// endpoint (built by the JS
/// host from the injected path); `None`/empty leaves proxying disabled. Called
/// once from `web_start`, before any request is issued.
pub fn init(url: Option<String>) {
    let url = url.filter(|u| !u.is_empty());
    let on = url.is_some();
    CONFIG.with(|c| *c.borrow_mut() = url);
    if on {
        socket::connect();
    }
}

pub fn enabled() -> bool {
    CONFIG.with(|c| c.borrow().is_some())
}

pub(crate) fn ws_url() -> Option<String> {
    CONFIG.with(|c| c.borrow().clone())
}

/// Issue a GitHub request: directly via fetch when disabled, otherwise over the
/// proxy socket. `url` is always `https://api.github.com{path}`.
pub async fn github_request(
    method: &str,
    url: &str,
    headers: &[(&str, String)],
    body: Option<String>,
) -> Result<HttpResponse, String> {
    if !enabled() {
        return fetch::request(method, url, headers, body).await;
    }
    socket::ensure_open().await?;
    let path = url
        .strip_prefix(API)
        .ok_or("proxy: refusing non-api.github.com URL")?;
    let id = NEXT_ID.with(|n| {
        let v = n.get();
        n.set(v.wrapping_add(1));
        v
    });
    let shared: Shared<Result<HttpResponse, String>> = new_shared();
    PENDING.with(|p| p.borrow_mut().insert(id, shared.clone()));
    if let Err(e) = socket::send(&envelope(id, method, path, headers, &body)) {
        PENDING.with(|p| {
            p.borrow_mut().remove(&id);
        });
        return Err(e);
    }
    Waiter(shared).await
}

/// Resolve the pending request a server reply belongs to.
pub(crate) fn dispatch(text: &str) {
    let Ok(v) = serde_json::from_str::<Value>(text) else {
        return;
    };
    let Some(id) = v.get("id").and_then(Value::as_u64) else {
        return;
    };
    let Some(shared) = PENDING.with(|p| p.borrow_mut().remove(&id)) else {
        return;
    };
    if let Some(err) = v.get("error").and_then(Value::as_str) {
        fulfill(&shared, Err(err.to_string()));
        return;
    }
    let status = v.get("status").and_then(Value::as_u64).unwrap_or(0) as u16;
    if status == 0 {
        fulfill(&shared, Err("proxy: malformed server reply".to_string()));
        return;
    }
    if let (Some(r), Some(l)) = (
        v.get("remaining").and_then(Value::as_u64),
        v.get("limit").and_then(Value::as_u64),
    ) {
        fetch::RATE_LIMIT.with(|c| c.set(Some((r as u32, l as u32))));
    }
    let body = v.get("body").and_then(Value::as_str).unwrap_or_default().to_string();
    fulfill(&shared, Ok(HttpResponse { status, body }));
}

/// Serialize one request for the wire: `{id, method, path, headers, body}`.
fn envelope(id: u64, method: &str, path: &str, headers: &[(&str, String)], body: &Option<String>) -> String {
    let h: serde_json::Map<String, Value> = headers
        .iter()
        .map(|(k, v)| (k.to_string(), Value::from(v.clone())))
        .collect();
    let mut o = serde_json::Map::new();
    o.insert("id".into(), Value::from(id));
    o.insert("method".into(), Value::from(method));
    o.insert("path".into(), Value::from(path));
    o.insert("headers".into(), Value::Object(h));
    o.insert("body".into(), body.clone().map_or(Value::Null, Value::from));
    Value::Object(o).to_string()
}

// --- request/response correlation: a one-shot future woken from JS callbacks ---

pub(crate) struct WaiterState<T> {
    result: Option<T>,
    waker: Option<Waker>,
}

struct Waiter<T>(Shared<T>);

impl<T> Future for Waiter<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut s = self.0.borrow_mut();
        match s.result.take() {
            Some(v) => Poll::Ready(v),
            None => {
                s.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

pub(crate) fn new_shared<T>() -> Shared<T> {
    Rc::new(RefCell::new(WaiterState { result: None, waker: None }))
}

/// Store a result and wake the awaiting task. The RefCell borrow is released
/// before `wake()` so a synchronously-polling executor can't re-borrow.
pub(crate) fn fulfill<T>(shared: &Shared<T>, val: T) {
    let waker = {
        let mut s = shared.borrow_mut();
        s.result = Some(val);
        s.waker.take()
    };
    if let Some(w) = waker {
        w.wake();
    }
}

/// Register interest in the socket reaching OPEN; the future resolves Ok on
/// open, or Err if the connection fails first.
pub(crate) fn register_open_waiter() -> impl Future<Output = Result<(), String>> {
    let shared = new_shared::<Result<(), String>>();
    OPEN_WAITERS.with(|w| w.borrow_mut().push(shared.clone()));
    Waiter(shared)
}
