//! Presentation primitives shared by the UI layer: the input model
//! (keys/mouse/line editing), grid geometry, the color theme, and the
//! hand-rolled per-language syntax highlighters. Pure logic, no web-sys —
//! `highlight` uses `crate::ui::input` intra-crate as before.

pub mod highlight;
pub mod ui;
