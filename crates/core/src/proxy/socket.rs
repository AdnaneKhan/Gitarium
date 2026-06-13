//! WebSocket transport for the proxy: connection lifecycle and the JS event
//! callbacks. All shared state lives in the parent module; this file owns only
//! the live socket and its closures (kept alive here so the callbacks fire).

use std::cell::RefCell;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

use super::{fulfill, State, OPEN_WAITERS, PENDING, STATE};

struct Conn {
    ws: WebSocket,
    _onopen: Closure<dyn FnMut()>,
    _onmessage: Closure<dyn FnMut(MessageEvent)>,
    _onclose: Closure<dyn FnMut()>,
    _onerror: Closure<dyn FnMut()>,
}

thread_local! {
    static CONN: RefCell<Option<Conn>> = RefCell::new(None);
}

/// Open a fresh socket. Called from `init` and, lazily, from `ensure_open` once
/// a previous socket has gone down — never from inside a JS callback, so
/// dropping the previous `Conn` (and its closures) here is safe.
pub(super) fn connect() {
    let Some(url) = super::ws_url() else {
        return;
    };
    // Replace any stale connection: detach its handlers first so a late event
    // can't fire into a half-dropped Conn, then let it drop.
    if let Some(old) = CONN.with(|c| c.borrow_mut().take()) {
        detach(&old.ws);
    }
    let ws = match WebSocket::new(&url) {
        Ok(ws) => ws,
        Err(_) => {
            STATE.with(|s| s.set(State::Disconnected));
            fail_all("proxy: could not open socket");
            return;
        }
    };

    let onopen = Closure::wrap(Box::new(|| {
        STATE.with(|s| s.set(State::Open));
        let waiters = OPEN_WAITERS.with(|w| std::mem::take(&mut *w.borrow_mut()));
        for w in waiters {
            fulfill(&w, Ok(()));
        }
    }) as Box<dyn FnMut()>);
    let onmessage = Closure::wrap(Box::new(|e: MessageEvent| {
        if let Some(txt) = e.data().as_string() {
            super::dispatch(&txt);
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    let onclose = Closure::wrap(Box::new(|| down("proxy: socket closed")) as Box<dyn FnMut()>);
    let onerror = Closure::wrap(Box::new(|| down("proxy: socket error")) as Box<dyn FnMut()>);

    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));

    STATE.with(|s| s.set(State::Connecting));
    CONN.with(|c| {
        *c.borrow_mut() = Some(Conn {
            ws,
            _onopen: onopen,
            _onmessage: onmessage,
            _onclose: onclose,
            _onerror: onerror,
        });
    });
}

/// Park until the socket is OPEN. Returns Err (hard-fail) if it can't connect.
pub(super) async fn ensure_open() -> Result<(), String> {
    match STATE.with(|s| s.get()) {
        State::Open => return Ok(()),
        State::Disconnected => connect(),
        State::Connecting => {}
    }
    // connect() can fail synchronously (e.g. a bad URL) → don't park forever.
    if STATE.with(|s| s.get()) == State::Disconnected {
        return Err("proxy: cannot reach server".to_string());
    }
    super::register_open_waiter().await
}

/// Send one already-serialized request frame; re-checks OPEN to close the
/// window between `ensure_open` and here.
pub(super) fn send(text: &str) -> Result<(), String> {
    CONN.with(|c| {
        let b = c.borrow();
        let conn = b.as_ref().ok_or("proxy: not connected")?;
        if conn.ws.ready_state() != WebSocket::OPEN {
            return Err("proxy: socket not open".to_string());
        }
        conn.ws.send_with_str(text).map_err(|_| "proxy: send failed".to_string())
    })
}

/// onclose/onerror: mark down and fail everything in flight. We deliberately do
/// NOT drop the `Conn` here — that would free this very closure mid-call; the
/// next `connect()` replaces it. Reconnect is lazy (the next request triggers).
fn down(msg: &str) {
    STATE.with(|s| s.set(State::Disconnected));
    fail_all(msg);
}

fn fail_all(msg: &str) {
    let pending = PENDING.with(|p| std::mem::take(&mut *p.borrow_mut()));
    for s in pending.into_values() {
        fulfill(&s, Err(msg.to_string()));
    }
    let open = OPEN_WAITERS.with(|w| std::mem::take(&mut *w.borrow_mut()));
    for s in open {
        fulfill(&s, Err(msg.to_string()));
    }
}

fn detach(ws: &WebSocket) {
    ws.set_onopen(None);
    ws.set_onmessage(None);
    ws.set_onclose(None);
    ws.set_onerror(None);
    let _ = ws.close();
}
