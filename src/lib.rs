//! RustVM — a GitHub client in Rust/WASM with a GPU-rendered browser UI
//! (px module: WebGL2, SDF shapes, multi-font atlas). The former terminal
//! mode was removed; a headless background-agent mode may replace it later.

mod agent;
mod app;
mod fetch;
mod github;
mod highlight;
mod px;
mod sh;
mod ui;
mod vfs;

use std::cell::RefCell;
use std::collections::VecDeque;

use wasm_bindgen::prelude::*;

use app::{App, Msg, Route};
use ui::input::{Event, Key, Mods};

#[wasm_bindgen]
extern "C" {
    /// Provided by the host on globalThis. Called when an async message
    /// lands so the host schedules a frame.
    #[wasm_bindgen(js_name = host_wake)]
    fn host_wake();
}

thread_local! {
    static HOST: RefCell<Option<Host>> = RefCell::new(None);
    static MSGS: RefCell<VecDeque<Msg>> = RefCell::new(VecDeque::new());
}

struct Host {
    app: App,
    renderer: px::gl::Renderer,
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

fn with_host<R>(f: impl FnOnce(&mut Host) -> R) -> Option<R> {
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
    let renderer = px::gl::Renderer::new(canvas_id).map_err(|e| JsValue::from_str(&e))?;
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

/// Returns true when the key was consumed (host should preventDefault).
#[wasm_bindgen]
pub fn web_key(key: &str, ctrl: bool, alt: bool, shift: bool) -> bool {
    // Ctrl/Cmd+C copies the active selection (agent transcript or editor)
    // to the system clipboard; falls through when nothing is selected.
    if ctrl && (key == "c" || key == "C") {
        let copied = with_host(|h| {
            let text = if h.app.route == Route::Agent {
                h.view.agent_selection_text()
            } else {
                h.app.editor_selection_text()
            };
            match text {
                Some(t) => {
                    copy_to_clipboard(&t);
                    h.app.toast = Some(("copied".into(), false));
                    h.app.dirty = true;
                    h.view.needs_frame = true;
                    true
                }
                None => false,
            }
        })
        .unwrap_or(false);
        if copied {
            return true;
        }
    }
    let Some(ev) = map_dom_key(key, ctrl, alt, shift) else {
        return false;
    };
    with_host(|h| {
        h.app.on_event(ev);
        h.view.needs_frame = true;
    });
    true
}

fn copy_to_clipboard(text: &str) {
    let Some(w) = web_sys::window() else { return };
    // Fire-and-forget; the promise resolves off the event loop.
    let _ = w.navigator().clipboard().write_text(text);
}

#[wasm_bindgen]
pub fn web_mouse_down(px_x: f32, px_y: f32) {
    with_host(|h| h.view.on_mouse_down(&mut h.app, px_x, px_y));
}

#[wasm_bindgen]
pub fn web_mouse_up(px_x: f32, px_y: f32) {
    with_host(|h| h.view.on_mouse_up(&mut h.app, px_x, px_y));
}

/// Legacy single-event click (kept for hosts without down/up wiring).
#[wasm_bindgen]
pub fn web_mouse(px_x: f32, px_y: f32) {
    web_mouse_down(px_x, px_y);
    web_mouse_up(px_x, px_y);
}

#[wasm_bindgen]
pub fn web_mouse_move(px_x: f32, px_y: f32) {
    with_host(|h| h.view.on_mouse_move(&mut h.app, px_x, px_y));
}

#[wasm_bindgen]
pub fn web_wheel(px_x: f32, px_y: f32, delta_y: f32) {
    with_host(|h| {
        h.view.mouse = (px_x, px_y);
        h.view.wheel(&mut h.app, px_x, px_y, delta_y);
    });
}

#[wasm_bindgen]
pub fn web_paste(text: &str) {
    with_host(|h| {
        h.app.on_event(Event::Paste(text.to_string()));
        h.view.needs_frame = true;
    });
}

/// CSS cursor for the canvas, polled by the host after mouse events.
#[wasm_bindgen]
pub fn web_cursor_style() -> String {
    with_host(|h| {
        if h.view.cursor_pointer {
            "pointer".to_string()
        } else if h.view.cursor_text {
            "text".to_string()
        } else {
            "default".to_string()
        }
    })
    .unwrap_or_else(|| "default".to_string())
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

fn map_dom_key(key: &str, ctrl: bool, alt: bool, shift: bool) -> Option<Event> {
    let mods = Mods { ctrl, alt, shift };
    let k = match key {
        "Enter" => Key::Enter,
        "Escape" => Key::Esc,
        "Tab" => {
            if shift {
                Key::BackTab
            } else {
                Key::Tab
            }
        }
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        "ArrowUp" => Key::Up,
        "ArrowDown" => Key::Down,
        "ArrowLeft" => Key::Left,
        "ArrowRight" => Key::Right,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        _ => {
            let mut chars = key.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None; // "Shift", "F5", ...
            }
            Key::Char(c)
        }
    };
    Some(Event::Key(k, mods))
}
