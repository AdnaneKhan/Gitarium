//! Parsing tool invocations out of assistant content and building the
//! tool_result blocks that answer them.

use serde_json::{json, Value};

pub enum ToolCall {
    Github { id: String, method: String, path: String, body: Option<String> },
    CodeSearch { id: String, query: String, repo: Option<String>, page: u32 },
    Bash { id: String, command: String },
    Grep { id: String, pattern: String, path: Option<String>, icase: bool },
    Find { id: String, pattern: String, path: Option<String> },
    Unknown { id: String, name: String },
}

impl ToolCall {
    pub fn id(&self) -> &str {
        match self {
            ToolCall::Github { id, .. }
            | ToolCall::CodeSearch { id, .. }
            | ToolCall::Bash { id, .. }
            | ToolCall::Grep { id, .. }
            | ToolCall::Find { id, .. }
            | ToolCall::Unknown { id, .. } => id,
        }
    }

    /// True for tool calls that change server state: any non-GET
    /// `github_api` request. The shell/search tools only read the in-memory
    /// VFS, so they never mutate anything. Used by the interactive app to
    /// gate writes behind manual approval (the headless agent ignores this).
    pub fn is_mutating(&self) -> bool {
        matches!(self, ToolCall::Github { method, .. } if !method.eq_ignore_ascii_case("GET"))
    }

    pub fn label(&self) -> String {
        let trunc = |s: &str| {
            let t: String = s.chars().take(70).collect();
            format!("{}{}", t, if t.len() < s.len() { "…" } else { "" })
        };
        match self {
            ToolCall::Github { method, path, .. } => format!("{} {}", method, path),
            ToolCall::CodeSearch { query, repo, .. } => format!(
                "search {}{}",
                trunc(query),
                repo.as_deref().map(|r| format!(" in {}", r)).unwrap_or_default()
            ),
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
                "code_search" => ToolCall::CodeSearch {
                    id,
                    query: input["query"].as_str().unwrap_or_default().to_string(),
                    repo: input["repo"].as_str().map(str::to_string),
                    page: input["page"].as_u64().map(|p| p.max(1) as u32).unwrap_or(1),
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

pub fn tool_result_block(id: &str, text: &str, ok: bool) -> Value {
    json!({
        "type": "tool_result",
        "tool_use_id": id,
        "content": text,
        "is_error": !ok,
    })
}
