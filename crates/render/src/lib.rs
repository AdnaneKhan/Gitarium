//! GPU rendering of the app state: WebGL/WebGL1 with a Canvas2D software
//! fallback, SDF shapes, and the multi-font glyph atlas. Depends on
//! `rustvm-app` (it draws that state) and re-imports the agent/foundation/ui
//! modules it displays so `px`'s `crate::app` / `crate::ui` / … paths resolve
//! unchanged.
use rustvm_agent::agent;
use rustvm_app::app;
use rustvm_core::{fetch, github};
use rustvm_ui::{highlight, ui};

pub mod px;
