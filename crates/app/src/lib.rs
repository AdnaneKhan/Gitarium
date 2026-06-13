//! The UI state machine (routes, async message handling, key/mouse
//! dispatch) plus the tiny async runtime that ferries `Msg` results back
//! into it. Pure logic — drawing lives in `rustvm-render`, which depends on
//! this crate. The agent/foundation/ui modules are re-imported under their
//! old names so the app's `crate::agent` / `crate::github` / `crate::ui`
//! paths resolve unchanged.

use std::cell::RefCell;
use std::collections::VecDeque;

use wasm_bindgen::prelude::*;

use rustvm_agent::agent;
use rustvm_core::{github, proxy, store};
use rustvm_ui::{highlight, ui};

pub mod app;

use app::Msg;

#[wasm_bindgen]
extern "C" {
    /// Provided by the host on globalThis. Called when an async message
    /// lands so the host schedules a frame. Headless targets never link
    /// this crate, so the import is absent there.
    #[wasm_bindgen(js_name = host_wake)]
    pub fn host_wake();
}

thread_local! {
    static MSGS: RefCell<VecDeque<Msg>> = RefCell::new(VecDeque::new());
}

/// Run a future to completion off the event loop and enqueue its message.
pub fn spawn_msg<F>(fut: F)
where
    F: std::future::Future<Output = Msg> + 'static,
{
    wasm_bindgen_futures::spawn_local(async move {
        let msg = fut.await;
        MSGS.with(|q| q.borrow_mut().push_back(msg));
        host_wake();
    });
}

/// Drain every queued async result into the app's state machine.
pub fn drain_msgs(app: &mut app::App) {
    loop {
        let m = MSGS.with(|q| q.borrow_mut().pop_front());
        match m {
            Some(m) => app.on_msg(m),
            None => break,
        }
    }
}
