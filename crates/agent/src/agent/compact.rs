//! Context-window management: usage accounting, the summarization request
//! that compacts a long conversation, and the replacement history it
//! produces. The shell VFS survives compaction, so stored API responses
//! and scratch files stay retrievable afterwards.

use serde_json::{json, Value};

use super::tools;

/// Compact once a turn's total context passes this: the 200k window minus
/// headroom for max_tokens (16k) and a few more turns of growth.
pub const SOFT_CAP: u64 = 160_000;

/// Char budget for the history replayed in the summary request (~4 chars
/// per token), so that request fits even when triggered by an overflow.
const SUMMARY_CHARS: usize = 480_000;

/// Total context consumed by the request+response behind `resp`.
pub fn context_tokens(resp: &Value) -> Option<u64> {
    let u = resp.get("usage")?;
    let n = |k: &str| u[k].as_u64().unwrap_or(0);
    Some(
        n("input_tokens")
            + n("cache_creation_input_tokens")
            + n("cache_read_input_tokens")
            + n("output_tokens"),
    )
}

/// True when an API error means the request exceeded the context window.
pub fn is_overflow(err: &str) -> bool {
    err.contains("prompt is too long") || err.contains("exceed context limit")
}

fn has_tool_result(msg: &Value) -> bool {
    msg["content"]
        .as_array()
        .map(|a| a.iter().any(|b| b["type"] == "tool_result"))
        .unwrap_or(false)
}

/// Newest suffix of `history` within budget, starting on a plain user
/// message (a leading tool_result would orphan its tool_use and 400).
fn trim(history: &[Value]) -> &[Value] {
    let mut start = history.len();
    let mut chars = 0;
    for (i, m) in history.iter().enumerate().rev() {
        chars += m.to_string().len();
        if chars > SUMMARY_CHARS {
            break;
        }
        start = i;
    }
    while start < history.len()
        && !(history[start]["role"] == "user" && !has_tool_result(&history[start]))
    {
        start += 1;
    }
    &history[start..]
}

/// The first user message's text — the original ask, re-stated in the
/// instruction when trimming dropped it.
fn first_text(history: &[Value]) -> Option<String> {
    let c = &history.first()?["content"];
    let s = c.as_str().or_else(|| {
        c.as_array()?.iter().find(|b| b["type"] == "text")?["text"].as_str()
    })?;
    Some(s.chars().take(2_000).collect())
}

/// Body of the summarization request: trimmed history plus the handoff
/// instruction, merged into a trailing user turn when one exists. Same
/// system/tools as a normal turn so the cached prefix is reused.
pub fn summary_request(model: &str, system: &str, history: &[Value]) -> String {
    let kept = trim(history);
    let mut text = String::from(super::prompts::get("compact_instruction"));
    if kept.len() < history.len() {
        if let Some(ask) = first_text(history) {
            text.push_str("\nThe original request was: ");
            text.push_str(&ask);
        }
    }
    let mut msgs = kept.to_vec();
    match msgs.last_mut() {
        Some(last) if last["role"] == "user" => {
            let block = json!({"type": "text", "text": text});
            match &mut last["content"] {
                Value::String(s) => {
                    let prev = std::mem::take(s);
                    last["content"] = json!([{"type": "text", "text": prev}, block]);
                }
                Value::Array(a) => a.push(block),
                _ => {}
            }
        }
        _ => msgs.push(json!({"role": "user", "content": text})),
    }
    json!({
        "model": model,
        "max_tokens": 8000,
        "thinking": {"type": "adaptive"},
        "cache_control": {"type": "ephemeral"},
        "system": system,
        "tools": tools::tools(),
        "tool_choice": {"type": "none"},
        "messages": msgs,
    })
    .to_string()
}

/// The single-message history that replaces the conversation.
pub fn compacted_history(summary: &str) -> Vec<Value> {
    vec![json!({
        "role": "user",
        "content": format!(
            "{}{}{}",
            super::prompts::get("compact_history_pre"),
            summary,
            super::prompts::get("compact_history_post"),
        ),
    })]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_all_usage_fields() {
        let resp = json!({"usage": {"input_tokens": 10, "cache_creation_input_tokens": 20,
            "cache_read_input_tokens": 30, "output_tokens": 5}});
        assert_eq!(context_tokens(&resp), Some(65));
        assert_eq!(context_tokens(&json!({})), None);
    }

    #[test]
    fn detects_overflow_errors() {
        assert!(is_overflow("HTTP 400: prompt is too long: 214315 tokens > 200000 maximum"));
        assert!(!is_overflow("HTTP 429: rate limited"));
    }

    #[test]
    fn instruction_merges_into_trailing_tool_result_turn() {
        let h = vec![
            json!({"role": "user", "content": "find failing checks"}),
            json!({"role": "assistant", "content":
                [{"type": "tool_use", "id": "t1", "name": "bash", "input": {}}]}),
            json!({"role": "user", "content":
                [{"type": "tool_result", "tool_use_id": "t1", "content": "x"}]}),
        ];
        let req: Value = serde_json::from_str(&summary_request("m", "sys", &h)).unwrap();
        assert_eq!(req["tool_choice"]["type"], "none");
        let msgs = req["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 3, "instruction must merge, not append");
        let blocks = msgs[2]["content"].as_array().unwrap();
        assert_eq!(blocks[0]["type"], "tool_result");
        assert!(blocks[1]["text"].as_str().unwrap().contains("handoff"));
    }

    #[test]
    fn trims_to_user_boundary_and_restates_original_ask() {
        let h = vec![
            json!({"role": "user", "content": "the original ask"}),
            json!({"role": "assistant", "content":
                [{"type": "text", "text": "x".repeat(SUMMARY_CHARS)}]}),
            json!({"role": "user", "content":
                [{"type": "tool_result", "tool_use_id": "t", "content": "r"}]}),
            json!({"role": "assistant", "content": [{"type": "text", "text": "done"}]}),
            json!({"role": "user", "content": "next"}),
        ];
        let req: Value = serde_json::from_str(&summary_request("m", "sys", &h)).unwrap();
        let msgs = req["messages"].as_array().unwrap();
        // Everything before the last plain user message was dropped: the
        // tool_result turn may not lead (its tool_use is gone).
        assert_eq!(msgs.len(), 1);
        let blocks = msgs[0]["content"].as_array().unwrap();
        assert_eq!(blocks[0]["text"], "next");
        assert!(blocks[1]["text"].as_str().unwrap().contains("the original ask"));
    }

    #[test]
    fn compacted_history_is_one_user_turn() {
        let h = compacted_history("did A, next B");
        assert_eq!(h.len(), 1);
        assert_eq!(h[0]["role"], "user");
        assert!(h[0]["content"].as_str().unwrap().contains("did A, next B"));
    }
}
