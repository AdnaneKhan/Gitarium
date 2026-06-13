//! Tool execution: the github_api proxy plus the in-memory shell tools.

use serde_json::Value;

use super::calls::ToolCall;
use crate::fetch;
use crate::github;
use crate::sh;
use crate::vfs;

/// API responses up to this size go straight into the tool result; larger
/// bodies are saved as files in the agent's virtual filesystem and
/// navigated with the bash tool, so nothing is truncated away.
const INLINE_LIMIT: usize = 2_000;

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

/// Cap any single tool result entering the conversation: outputs this
/// large come from bash/grep over VFS files, so the data stays in the
/// VFS and the cut is recoverable with a narrower query.
const RESULT_LIMIT: usize = 24_000;

pub(super) fn clip(text: String) -> String {
    let n = text.chars().count();
    if n <= RESULT_LIMIT {
        return text;
    }
    let kept = RESULT_LIMIT - 2_000;
    let cut: String = text.chars().take(kept).collect();
    format!(
        "{}\n[…clipped: {} of {} chars shown — the data is still in the VFS; \
         narrow it with head/grep/jq]",
        cut, kept, n
    )
}

/// Execute one tool call. Returns (result text, ok).
pub async fn exec(token: &Option<String>, call: &ToolCall) -> (String, bool) {
    let (text, ok) = match call {
        ToolCall::Github { method, path, body, .. } => exec_github(token, method, path, body).await,
        ToolCall::CodeSearch { query, repo, page, .. } => {
            exec_code_search(token, query, repo.as_deref(), *page).await
        }
        ToolCall::Bash { command, .. } => sh::run(command),
        ToolCall::Grep { pattern, path, icase, .. } => sh::tool_grep(pattern, path.as_deref(), *icase),
        ToolCall::Find { pattern, path, .. } => sh::tool_find(path.as_deref(), pattern),
        ToolCall::Unknown { name, .. } => (format!("unknown tool '{}'", name), false),
    };
    (clip(text), ok)
}

async fn exec_code_search(
    token: &Option<String>,
    query: &str,
    repo: Option<&str>,
    page: u32,
) -> (String, bool) {
    if token.is_none() {
        return (
            "code search requires GitHub authentication — this session is anonymous".into(),
            false,
        );
    }
    let q = match repo {
        Some(r) => format!("{} repo:{}", query, r),
        None => query.to_string(),
    };
    match github::search_code_global(token, &q, page).await {
        Ok(res) => (format_code_search(&res, page.max(1)), true),
        Err(e) => (format!("code search failed: {}", e), false),
    }
}

pub(super) fn format_code_search(res: &github::CodeSearch, page: u32) -> String {
    let mut out = format!(
        "{}{} matching files · page {}\n",
        if res.incomplete { "~" } else { "" },
        res.total,
        page
    );
    for it in &res.items {
        format_match(&mut out, it);
    }
    if res.items.is_empty() {
        out.push_str("no matches — broaden the query or drop qualifiers\n");
    } else if res.total > page as u64 * 30 {
        out.push_str("…more pages exist (pass page=N); narrow with path:/language:/filename: qualifiers\n");
    }
    out
}

fn format_match(out: &mut String, it: &github::CodeMatch) {
    out.push_str(&format!("{} {}\n", it.repo, it.path));
    for l in &it.lines {
        let line: String = l.chars().take(160).collect();
        out.push_str("  > ");
        out.push_str(&line);
        out.push('\n');
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
