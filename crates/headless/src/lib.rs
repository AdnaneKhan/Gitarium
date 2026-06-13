//! Headless-agent wasm target. Exposes a single export, `agent_run_headless`,
//! that drives `rustvm_agent::agent::headless::run` to completion — the same
//! agent loop, tools, knowledge bundle, and shell VFS as the in-app window,
//! but with no rendering linked in. Driven by `agent-headless.ts`.

use serde_json::Value;
use wasm_bindgen::prelude::*;

use rustvm_agent::agent::{headless, normalize_base};

/// Drive the agent toward `goal`, streaming JSON event strings through
/// `emit` (one string argument per call); resolves to a JSON
/// `{"outcome","detail"}` string. `github_token` and `base_url` are optional
/// (anonymous access / default endpoint when unset); a `max_turns` of 0 means
/// "use the default cap".
#[wasm_bindgen]
pub async fn agent_run_headless(
    goal: String,
    github_token: Option<String>,
    api_key: String,
    base_url: Option<String>,
    max_turns: u32,
    emit: js_sys::Function,
) -> Result<JsValue, JsValue> {
    let base = base_url.as_deref().and_then(normalize_base);
    let sink = move |v: &Value| {
        let _ = emit.call1(&JsValue::NULL, &JsValue::from_str(&v.to_string()));
    };
    let cap = if max_turns == 0 { 60 } else { max_turns };
    let (status, detail) =
        headless::run(&goal, github_token, &api_key, base.as_deref(), cap, &sink).await;
    let out = serde_json::json!({"outcome": status.as_str(), "detail": detail});
    Ok(JsValue::from_str(&out.to_string()))
}
