//! Claude-powered GitHub agent: raw Anthropic Messages API over
//! globalThis.fetch (there is no official Rust SDK; in the browser the
//! `anthropic-dangerous-direct-browser-access` header enables CORS).
//! The agent gets a single generic `github_api` tool so it can drive any
//! REST v3 endpoint on its own; the loop itself lives in app/mod.rs.

use serde_json::{json, Value};

use crate::fetch;
use crate::sh;
use crate::vfs;

pub const MODEL: &str = "claude-opus-4-8";
const DEFAULT_BASE: &str = "https://api.anthropic.com";
const API_VERSION: &str = "2023-06-01";
/// API responses up to this size go straight into the tool result; larger
/// bodies are saved as files in the agent's virtual filesystem and
/// navigated with the bash tool, so nothing is truncated away.
const INLINE_LIMIT: usize = 2_000;

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
         Replies render in a small terminal-style window: keep them short, lead \
         with the outcome, and use plain text — no markdown except ``` fences \
         (with a language tag) for code or file contents.",
    );
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

fn tools() -> Value {
    json!([
        {
            "name": "github_api",
            "description": "Execute one GitHub REST v3 API request, authenticated as the user. \
                Call this whenever you need to read or change anything on GitHub — repos, \
                issues, pull requests, file contents, branches, refs, actions, releases, \
                users, search. `path` starts with '/' and may include a query string, e.g. \
                /repos/OWNER/REPO/issues?state=open&per_page=100. `body` is the JSON request \
                body for POST/PUT/PATCH. Small responses come back inline; large JSON \
                responses are stored and you get an id plus a shape summary — inspect those \
                with query_response.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "method": {
                        "type": "string",
                        "enum": ["GET", "POST", "PUT", "PATCH", "DELETE"],
                        "description": "HTTP method"
                    },
                    "path": {
                        "type": "string",
                        "description": "API path starting with '/', optionally with a query string"
                    },
                    "body": {
                        "type": "object",
                        "description": "JSON request body, for POST/PUT/PATCH"
                    }
                },
                "required": ["method", "path"]
            }
        },
        {
            "name": "bash",
            "description": "Run a command in this session's minimal in-memory shell — use it to \
                navigate saved API responses and keep notes on long tasks. There is no real OS: \
                only a virtual filesystem holding the /rN.json files saved by github_api and \
                anything you write. The ONLY available commands are: ls, cat, head, tail, grep \
                (-i -n -v -c -r), wc (-l -w -c), sort (-r -n -u), uniq (-c), cut (-d -f), find \
                (-name), echo (-n), rm, mkdir, touch, pwd, help, and jq (the FULL jq language \
                via jaq; -r for raw strings; single-quote filters). Syntax: pipes |, redirects \
                > >> <, sequencing ; and &&. NOT available: shell variables, $(…) substitution, \
                glob expansion in arguments (use find -name or grep -r), cd, loops, ||, sed, \
                awk, xargs, and any network access — GitHub calls go through github_api. Run \
                'help' any time to re-check. Examples: \"cat /r1.json | jq -r '.[] | \
                select(.fork == false) | .full_name' | head -20\", \"grep -in 'error' \
                /r2.json\", \"jq '.items | group_by(.user.login) | map({user: .[0].user.login, \
                n: length})' /r3.json\".",
            "input_schema": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command line to run"
                    }
                },
                "required": ["command"]
            }
        },
        {
            "name": "grep",
            "description": "Search file contents in the session's virtual filesystem with a \
                regular expression — saved API responses (/rN.json) and your scratch files. \
                Returns matching lines as path:line:text. The pattern is taken verbatim (no \
                shell quoting needed), so prefer this over bash grep for patterns containing \
                quotes, $ anchors, or backslashes. Searches every file unless `path` narrows \
                it to one file or directory. For structured JSON queries, jq via the bash \
                tool is usually sharper.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regular expression to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "File or directory to search (default: all files)"
                    },
                    "ignore_case": {
                        "type": "boolean",
                        "description": "Case-insensitive matching"
                    }
                },
                "required": ["pattern"]
            }
        },
        {
            "name": "find",
            "description": "List files in the session's virtual filesystem whose name matches \
                a glob (* and ?) — e.g. \"*.json\" for all saved API responses. Returns full \
                paths. Use `path` to limit the search to a directory.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Filename glob, e.g. \"*.json\""
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search under (default /)"
                    }
                },
                "required": ["pattern"]
            }
        }
    ])
}

/// Wipe the agent's virtual filesystem (CLEAR chip / new session).
pub fn clear_store() {
    vfs::clear();
}

/// One-line structural description so the agent knows what to query for.
fn shape(v: &Value) -> String {
    fn keys_preview(o: &serde_json::Map<String, Value>) -> String {
        let mut s = o.keys().take(30).cloned().collect::<Vec<_>>().join(", ");
        if o.len() > 30 {
            s.push_str(", …");
        }
        s
    }
    match v {
        Value::Array(a) => {
            let mut s = format!("array of {} items", a.len());
            if let Some(Value::Object(o)) = a.first() {
                s.push_str(&format!("; item keys: {}", keys_preview(o)));
            }
            s
        }
        Value::Object(o) => format!("object with keys: {}", keys_preview(o)),
        _ => "scalar".to_string(),
    }
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
        "tools": tools(),
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
// Tool execution
// ---------------------------------------------------------------------------

pub enum ToolCall {
    Github { id: String, method: String, path: String, body: Option<String> },
    Bash { id: String, command: String },
    Grep { id: String, pattern: String, path: Option<String>, icase: bool },
    Find { id: String, pattern: String, path: Option<String> },
    Unknown { id: String, name: String },
}

impl ToolCall {
    pub fn id(&self) -> &str {
        match self {
            ToolCall::Github { id, .. }
            | ToolCall::Bash { id, .. }
            | ToolCall::Grep { id, .. }
            | ToolCall::Find { id, .. }
            | ToolCall::Unknown { id, .. } => id,
        }
    }

    pub fn label(&self) -> String {
        let trunc = |s: &str| {
            let t: String = s.chars().take(70).collect();
            format!("{}{}", t, if t.len() < s.len() { "…" } else { "" })
        };
        match self {
            ToolCall::Github { method, path, .. } => format!("{} {}", method, path),
            ToolCall::Bash { command, .. } => format!("$ {}", trunc(command)),
            ToolCall::Grep { pattern, path, .. } => format!(
                "grep {}{}",
                trunc(pattern),
                path.as_deref().map(|p| format!(" {}", p)).unwrap_or_default()
            ),
            ToolCall::Find { pattern, path, .. } => format!(
                "find {}{}",
                trunc(pattern),
                path.as_deref().map(|p| format!(" {}", p)).unwrap_or_default()
            ),
            ToolCall::Unknown { name, .. } => name.clone(),
        }
    }
}

/// Extract tool invocations from an assistant content array.
pub fn parse_tool_calls(content: &Value) -> Vec<ToolCall> {
    let Some(blocks) = content.as_array() else {
        return Vec::new();
    };
    blocks
        .iter()
        .filter(|b| b["type"] == "tool_use")
        .map(|b| {
            let id = b["id"].as_str().unwrap_or_default().to_string();
            let input = &b["input"];
            match b["name"].as_str().unwrap_or_default() {
                "github_api" => ToolCall::Github {
                    id,
                    method: input["method"].as_str().unwrap_or("GET").to_string(),
                    path: input["path"].as_str().unwrap_or_default().to_string(),
                    body: (!input["body"].is_null()).then(|| input["body"].to_string()),
                },
                "bash" => ToolCall::Bash {
                    id,
                    command: input["command"].as_str().unwrap_or_default().to_string(),
                },
                "grep" => ToolCall::Grep {
                    id,
                    pattern: input["pattern"].as_str().unwrap_or_default().to_string(),
                    path: input["path"].as_str().map(str::to_string),
                    icase: input["ignore_case"].as_bool().unwrap_or(false),
                },
                "find" => ToolCall::Find {
                    id,
                    pattern: input["pattern"].as_str().unwrap_or_default().to_string(),
                    path: input["path"].as_str().map(str::to_string),
                },
                name => ToolCall::Unknown { id, name: name.to_string() },
            }
        })
        .collect()
}

/// Execute one tool call. Returns (result text, ok).
pub async fn exec(token: &Option<String>, call: &ToolCall) -> (String, bool) {
    match call {
        ToolCall::Github { method, path, body, .. } => exec_github(token, method, path, body).await,
        ToolCall::Bash { command, .. } => sh::run(command),
        ToolCall::Grep { pattern, path, icase, .. } => sh::tool_grep(pattern, path.as_deref(), *icase),
        ToolCall::Find { pattern, path, .. } => sh::tool_find(path.as_deref(), pattern),
        ToolCall::Unknown { name, .. } => (format!("unknown tool '{}'", name), false),
    }
}

async fn exec_github(
    token: &Option<String>,
    method: &str,
    path: &str,
    body: &Option<String>,
) -> (String, bool) {
    if !path.starts_with('/') {
        return ("error: path must start with '/'".to_string(), false);
    }
    let mut headers: Vec<(&str, String)> = vec![
        ("Accept", "application/vnd.github+json".to_string()),
        ("X-GitHub-Api-Version", "2022-11-28".to_string()),
    ];
    if let Some(t) = token {
        headers.push(("Authorization", format!("Bearer {}", t)));
    }
    if body.is_some() {
        headers.push(("Content-Type", "application/json".to_string()));
    }
    let url = format!("https://api.github.com{}", path);
    match fetch::request(method, &url, &headers, body.clone()).await {
        Ok(r) => {
            let ok = (200..300).contains(&r.status);
            let chars = r.body.chars().count();
            if chars <= INLINE_LIMIT {
                return (format!("HTTP {}\n{}", r.status, r.body), ok);
            }
            // Big response: save it as a file in the agent's VFS and hand
            // back the path + a shape summary; it navigates with bash.
            match serde_json::from_str::<Value>(&r.body) {
                Ok(v) => {
                    let path = vfs::store_response(&r.body, "json");
                    (
                        format!(
                            "HTTP {} · saved to {} ({} chars) · {}\nExplore it with the bash tool, e.g. cat {} | jq '…'",
                            r.status,
                            path,
                            chars,
                            shape(&v),
                            path
                        ),
                        ok,
                    )
                }
                // Non-JSON (raw media types): still saved — grep-able.
                Err(_) => {
                    let path = vfs::store_response(&r.body, "txt");
                    (
                        format!(
                            "HTTP {} · non-JSON response saved to {} ({} chars)\nExplore it with the bash tool (grep, head, …)",
                            r.status, path, chars
                        ),
                        ok,
                    )
                }
            }
        }
        Err(e) => (format!("request failed: {}", e), false),
    }
}

pub fn tool_result_block(id: &str, text: &str, ok: bool) -> Value {
    json!({
        "type": "tool_result",
        "tool_use_id": id,
        "content": text,
        "is_error": !ok,
    })
}
