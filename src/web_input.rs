//! Browser-host input exports: keyboard, clipboard, mouse, wheel.

use wasm_bindgen::prelude::*;

use crate::with_host;
use rustvm_app::app::Route;
use rustvm_app::host_wake;
use rustvm_ui::ui::input::{Event, Key, Mods};

/// Returns true when the key was consumed (host should preventDefault).
#[wasm_bindgen]
pub fn web_key(key: &str, ctrl: bool, alt: bool, shift: bool) -> bool {
    // Ctrl/Cmd+C copies the active selection (agent transcript or editor)
    // to the system clipboard; falls through when nothing is selected.
    // With an overlay open neither selection is visible — copying it
    // would silently grab text from underneath, so don't.
    if ctrl && (key == "c" || key == "C") {
        let copied = with_host(|h| {
            if h.app.overlay.is_some() {
                return false;
            }
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
        let used = h.app.on_event(ev);
        h.view.needs_frame = true;
        used
    })
    .unwrap_or(false)
}

fn copy_to_clipboard(text: &str) {
    let Some(w) = web_sys::window() else { return };
    let promise = w.navigator().clipboard().write_text(text);
    // The eager "copied" toast is replaced if the promise rejects —
    // a dropped promise would let the toast lie about failures.
    wasm_bindgen_futures::spawn_local(async move {
        if wasm_bindgen_futures::JsFuture::from(promise).await.is_err() {
            with_host(|h| {
                h.app.toast = Some(("copy failed".into(), true));
                h.app.dirty = true;
                h.view.needs_frame = true;
            });
            host_wake();
        }
    });
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

/// Right-click: open the tree context menu (or dismiss it).
#[wasm_bindgen]
pub fn web_context_menu(px_x: f32, px_y: f32) {
    with_host(|h| h.view.on_context_menu(&mut h.app, px_x, px_y));
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
