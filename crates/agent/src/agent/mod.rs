//! Claude-powered GitHub agent: raw Anthropic Messages API over
//! globalThis.fetch (there is no official Rust SDK; in the browser the
//! `anthropic-dangerous-direct-browser-access` header enables CORS).
//! The agent drives the REST v3 API through a generic `github_api` tool,
//! explores code with a dedicated `code_search` tool, and navigates saved
//! responses with an in-memory shell (bash/grep/find); the loop itself
//! lives in app.

mod calls;
pub mod compact;
mod exec;
pub mod headless;
mod prompts;
#[cfg(test)]
mod tests;
mod tools;

pub use calls::{parse_tool_calls, tool_result_block, ToolCall};
pub use exec::exec;

use serde_json::{json, Value};

use crate::fetch;
use crate::vfs;

pub const MODEL: &str = "claude-opus-4-8";
const DEFAULT_BASE: &str = "https://api.anthropic.com";
const API_VERSION: &str = "2023-06-01";

const STORAGE_KEY: &str = "gitarium_anthropic_key";
const STORAGE_URL: &str = "gitarium_anthropic_url";
const STORAGE_MODEL: &str = "gitarium_model";

// ---------------------------------------------------------------------------
// API key persistence (same localStorage scheme as the GitHub PAT)
// ---------------------------------------------------------------------------

pub fn load_key() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    storage
        .get_item(STORAGE_KEY)
        .ok()?
        .filter(|k| !k.trim().is_empty())
}

pub fn save_key(key: &str) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item(STORAGE_KEY, key);
    }
}

pub fn clear_key() {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.remove_item(STORAGE_KEY);
    }
}

pub fn load_url() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    storage
        .get_item(STORAGE_URL)
        .ok()?
        .filter(|u| !u.trim().is_empty())
}

pub fn save_url(url: &str) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item(STORAGE_URL, url);
    }
}

pub fn clear_url() {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.remove_item(STORAGE_URL);
    }
}

/// The selected model id (persisted across sessions); `None` falls back to
/// `MODEL`. The id is never shown in the UI — only chosen via the picker.
pub fn load_model() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    storage
        .get_item(STORAGE_MODEL)
        .ok()?
        .filter(|m| !m.trim().is_empty())
}

pub fn save_model(model: &str) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item(STORAGE_MODEL, model);
    }
}

/// Clean up a user-entered endpoint base: trim, drop trailing slashes,
/// assume https when no scheme was given. Empty input means "use default".
pub fn normalize_base(input: &str) -> Option<String> {
    let t = input.trim().trim_end_matches('/');
    if t.is_empty() {
        return None;
    }
    if t.starts_with("http://") || t.starts_with("https://") {
        Some(t.to_string())
    } else {
        Some(format!("https://{}", t))
    }
}

/// Messages endpoint for an optional base override. Accepts both a bare
/// base (`https://proxy.corp`) and a full path already ending in
/// /v1/messages.
fn messages_url(base: Option<&str>) -> String {
    let base = base.unwrap_or(DEFAULT_BASE).trim_end_matches('/');
    if base.ends_with("/v1/messages") {
        base.to_string()
    } else {
        format!("{}/v1/messages", base)
    }
}

// ---------------------------------------------------------------------------
// Request construction
// ---------------------------------------------------------------------------

pub fn system_prompt(
    login: Option<&str>,
    repo: Option<(&str, &str)>,
    file: Option<&str>,
) -> String {
    let mut s = String::from(prompts::get("system"));
    s.push_str(&crate::knowledge::prompt_block());
    match login {
        Some(l) => s.push_str(&format!("\nAuthenticated as {}.", l)),
        None => s.push_str("\nAnonymous session: unauthenticated requests only, writes will fail."),
    }
    if let Some((full, branch)) = repo {
        s.push_str(&format!("\nUser is currently viewing repo {} on branch {}.", full, branch));
        if let Some(p) = file {
            s.push_str(&format!(" Open file: {}.", p));
        }
    }
    s
}

pub fn build_request(model: &str, system: &str, history: &[Value]) -> String {
    json!({
        "model": model,
        "max_tokens": 16000,
        "thinking": {"type": "adaptive"},
        // Auto-cache the deepest prefix: in a tool loop every turn replays
        // the whole conversation, so reads land on all but the newest turn.
        "cache_control": {"type": "ephemeral"},
        "system": system,
        "tools": tools::tools(),
        "messages": history,
    })
    .to_string()
}

// ---------------------------------------------------------------------------
// Messages API call
// ---------------------------------------------------------------------------

pub async fn complete(api_key: &str, base: Option<&str>, body: String) -> Result<Value, String> {
    let headers: Vec<(&str, String)> = vec![
        ("content-type", "application/json".to_string()),
        ("x-api-key", api_key.to_string()),
        ("anthropic-version", API_VERSION.to_string()),
        ("anthropic-dangerous-direct-browser-access", "true".to_string()),
    ];
    let resp = fetch::request("POST", &messages_url(base), &headers, Some(body)).await?;
    let v: Value = serde_json::from_str(&resp.body)
        .map_err(|_| format!("HTTP {}: unparseable API response", resp.status))?;
    if resp.status != 200 {
        let msg = v["error"]["message"].as_str().unwrap_or("unknown error");
        return Err(format!("HTTP {}: {}", resp.status, msg));
    }
    Ok(v)
}

// ---------------------------------------------------------------------------
// Model discovery — list the provider's models so the user can pick one
// instead of hardcoding a per-provider mapping.
// ---------------------------------------------------------------------------

/// One selectable model. `display` falls back to the id when the provider
/// doesn't give a human name (OpenAI-style `/models` only returns ids).
#[derive(Clone)]
pub struct ModelInfo {
    pub id: String,
    pub display: String,
}

/// Candidate endpoint roots to probe for a model list. Strips a trailing
/// `/v1/messages`, and — for an Anthropic-compatibility base like DeepSeek's
/// `…/anthropic` (which proxies /v1/messages but has no model list) — also
/// offers the provider root, where the OpenAI-style `/models` lives.
fn model_roots(base: Option<&str>) -> Vec<String> {
    let b = base
        .unwrap_or(DEFAULT_BASE)
        .trim_end_matches('/')
        .trim_end_matches("/v1/messages")
        .trim_end_matches('/');
    let mut roots = vec![b.to_string()];
    if let Some(parent) = b.strip_suffix("/anthropic").map(|p| p.trim_end_matches('/')) {
        if !parent.is_empty() && parent != b {
            roots.push(parent.to_string());
        }
    }
    roots
}

/// List the provider's models. For each candidate root tries the Anthropic
/// shape (`/v1/models`, `data:[{id, display_name}]`) then the OpenAI-compatible
/// shape (`/models`, `data:[{id}]`) — covering Anthropic, DeepSeek (incl. its
/// `…/anthropic` base), and the like with no client-side model table. Sends
/// both auth header styles so either provider family accepts it.
pub async fn list_models(api_key: &str, base: Option<&str>) -> Result<Vec<ModelInfo>, String> {
    let headers: Vec<(&str, String)> = vec![
        ("x-api-key", api_key.to_string()),
        ("authorization", format!("Bearer {}", api_key)),
        ("anthropic-version", API_VERSION.to_string()),
        ("anthropic-dangerous-direct-browser-access", "true".to_string()),
    ];
    let mut last_err = "model listing not supported by this endpoint".to_string();
    for root in model_roots(base) {
        for path in ["/v1/models", "/models"] {
            let url = format!("{}{}", root, path);
            match fetch::request("GET", &url, &headers, None).await {
                Ok(resp) if (200..300).contains(&resp.status) => {
                    let models = parse_models(&resp.body);
                    if !models.is_empty() {
                        return Ok(models);
                    }
                }
                Ok(resp) => last_err = format!("HTTP {}", resp.status),
                Err(e) => last_err = e,
            }
        }
    }
    Err(last_err)
}

fn parse_models(body: &str) -> Vec<ModelInfo> {
    let Ok(v) = serde_json::from_str::<Value>(body) else { return Vec::new() };
    let Some(arr) = v["data"].as_array() else { return Vec::new() };
    arr.iter()
        .filter_map(|m| {
            let id = m["id"].as_str()?.to_string();
            let display = m["display_name"].as_str().unwrap_or(&id).to_string();
            Some(ModelInfo { id, display })
        })
        .collect()
}

/// Wipe the agent's virtual filesystem (CLEAR chip / new session).
pub fn clear_store() {
    vfs::clear();
}
