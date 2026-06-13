//! RustVM — a GitHub client in Rust/WASM with a GPU-rendered browser UI
//! (px module: WebGL2/WebGL1 with a Canvas2D software fallback, SDF
//! shapes, multi-font atlas). The former terminal mode was removed; a
//! headless background-agent mode may replace it later.

mod agent;
mod app;
mod fetch;
mod github;
mod highlight;
mod knowledge;
mod px;
mod sh;
mod ui;
mod vfs;
mod web_input;

use std::cell::RefCell;
use std::collections::VecDeque;

use wasm_bindgen::prelude::*;

use app::{App, Msg};

#[wasm_bindgen]
extern "C" {
    /// Provided by the host on globalThis. Called when an async message
    /// lands so the host schedules a frame.
    #[wasm_bindgen(js_name = host_wake)]
    pub(crate) fn host_wake();
}

thread_local! {
    static HOST: RefCell<Option<Host>> = RefCell::new(None);
    static MSGS: RefCell<VecDeque<Msg>> = RefCell::new(VecDeque::new());
}

struct Host {
    app: App,
    renderer: px::render::Renderer,
    view: px::view::View,
}

/// Run a future to completion off the event loop and enqueue its message.
pub(crate) fn spawn_msg<F>(fut: F)
where
    F: std::future::Future<Output = Msg> + 'static,
{
    wasm_bindgen_futures::spawn_local(async move {
        let msg = fut.await;
        MSGS.with(|q| q.borrow_mut().push_back(msg));
        host_wake();
    });
}

pub(crate) fn with_host<R>(f: impl FnOnce(&mut Host) -> R) -> Option<R> {
    HOST.with(|h| h.borrow_mut().as_mut().map(f))
}

fn drain_msgs(app: &mut App) {
    loop {
        let m = MSGS.with(|q| q.borrow_mut().pop_front());
        match m {
            Some(m) => app.on_msg(m),
            None => break,
        }
    }
}

// ---------------------------------------------------------------------------
// Legacy demo export (kept from the original project)
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub async fn fetch_url(url: String) -> Result<String, JsValue> {
    let resp = fetch::request("GET", &url, &[], None)
        .await
        .map_err(|e| JsValue::from_str(&e))?;
    if !(200..300).contains(&resp.status) {
        return Err(JsValue::from_str(&format!("HTTP {}", resp.status)));
    }
    Ok(resp.body)
}

// ---------------------------------------------------------------------------
// Browser host
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn web_start(canvas_id: &str, font_px: f32, token: Option<String>) -> Result<(), JsValue> {
    knowledge::seed();
    let renderer = px::render::Renderer::new(canvas_id).map_err(|e| JsValue::from_str(&e))?;
    let view = px::view::View::new(font_px / 15.0);
    let host = Host { app: App::new(token), renderer, view };
    HOST.with(|h| *h.borrow_mut() = Some(host));
    web_frame(0.0);
    Ok(())
}

fn render(h: &mut Host, t_ms: f64, force: bool) {
    drain_msgs(&mut h.app);
    if !(force || h.app.dirty || h.view.needs_frame || h.view.is_active()) {
        return;
    }
    h.app.dirty = false;
    h.view.needs_frame = false;
    let (w, hh) = h.renderer.size();
    h.view
        .frame(&mut h.app, &mut h.renderer.dl, &mut h.renderer.atlas, w, hh, t_ms);
    h.renderer.flush();
}

/// Driven by requestAnimationFrame with its timestamp.
#[wasm_bindgen]
pub fn web_frame(t_ms: f64) {
    with_host(|h| render(h, t_ms, false));
}

#[wasm_bindgen]
pub fn web_resize(_w_px: f32, _h_px: f32) {
    with_host(|h| {
        h.app.dirty = true;
        h.view.needs_frame = true;
    });
}

/// Re-derive the UI scale after a devicePixelRatio change (browser zoom,
/// monitor move); glyphs re-rasterize at the new size on the next frame.
#[wasm_bindgen]
pub fn web_set_font_px(font_px: f32) {
    with_host(|h| {
        h.view.scale = font_px / 15.0;
        h.app.dirty = true;
        h.view.needs_frame = true;
    });
}

/// Text dump of everything drawn last frame — used by the test harness.
#[wasm_bindgen]
pub fn web_debug_text() -> String {
    with_host(|h| {
        let t = h.view.time() + 16.0;
        render(h, t, true);
        h.renderer.dl.dbg.join("\n")
    })
    .unwrap_or_default()
}

