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
#[cfg(test)]
mod tests;
mod tools;

pub use calls::{parse_tool_calls, tool_result_block};
pub use exec::exec;

use serde_json::{json, Value};

use crate::fetch;
use crate::vfs;

pub const MODEL: &str = "claude-opus-4-8";
const DEFAULT_BASE: &str = "https://api.anthropic.com";
const API_VERSION: &str = "2023-06-01";

const STORAGE_KEY: &str = "rustvm_anthropic_key";
const STORAGE_URL: &str = "rustvm_anthropic_url";

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
    let mut s = String::from(
        "You are an autonomous GitHub operations agent embedded in RustVM, a \
         GPU-rendered GitHub client. Operate the GitHub REST v3 API through the \
         github_api tool; chain as many calls as the task needs without asking \
         permission. Look up anything you are unsure about (schemas, ids, shas) \
         with extra GET calls instead of guessing.\n\
         On GET requests that return lists, always set per_page=100 when the \
         endpoint supports it, and fetch page=2, page=3, … while full pages \
         keep coming.\n\
         Large API responses are not returned inline: they are saved as files \
         (/r1.json, /r2.json, …) in an in-memory shell, and you get the path \
         plus a shape summary. Navigate them with the bash tool (pipes, \
         redirects, full jq) and the grep/find tools instead of re-fetching; \
         use scratch files for notes on long tasks.\n\
         To explore code, reach for code_search before fetching trees and \
         files blindly: it finds definitions, usages, and examples across \
         GitHub or within one repo (default branches only, ~10 searches/min — \
         make queries specific).\n\
         Replies render in a small terminal-style window: keep them short, lead \
         with the outcome, and use plain text — no markdown except ``` fences \
         (with a language tag) for code or file contents.",
    );
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

pub fn build_request(system: &str, history: &[Value]) -> String {
    json!({
        "model": MODEL,
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

/// Wipe the agent's virtual filesystem (CLEAR chip / new session).
pub fn clear_store() {
    vfs::clear();
}
