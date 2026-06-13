//! `gitarium` — the browser (web + interactive agent) wasm target. Wires the
//! app state machine (gitarium-app) to the GPU renderer (gitarium-render) behind
//! a `Host`, and exports the `web_*` entrypoints the JS host drives. Pure
//! functionality lives in the workspace crates; this crate is just the
//! browser entrypoint. The headless-agent target is a separate cdylib
//! (crates/headless) so each wasm bundles only what it needs.

mod web_input;

use std::cell::RefCell;

use wasm_bindgen::prelude::*;

use gitarium_app::app::App;
use gitarium_app::drain_msgs;
use gitarium_core::knowledge;
use gitarium_render::px;

thread_local! {
    static HOST: RefCell<Option<Host>> = RefCell::new(None);
}

pub(crate) struct Host {
    pub app: App,
    pub renderer: px::render::Renderer,
    pub view: px::view::View,
}

pub(crate) fn with_host<R>(f: impl FnOnce(&mut Host) -> R) -> Option<R> {
    HOST.with(|h| h.borrow_mut().as_mut().map(f))
}

#[wasm_bindgen]
pub fn web_start(
    canvas_id: &str,
    font_px: f32,
    token: Option<String>,
    proxy_url: Option<String>,
) -> Result<(), JsValue> {
    knowledge::seed();
    // Enable the GitHub API proxy when the host passes a ws(s):// endpoint
    // (server-injected); absent → calls go straight to api.github.com.
    gitarium_core::proxy::init(proxy_url);
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
