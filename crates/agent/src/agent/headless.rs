//! Headless autonomous agent: the same Messages-API loop as the in-app `i`
//! window — identical tools, knowledge bundle, shell VFS, and compaction —
//! but UI-free and self-driving. Given a goal it works without a human in
//! the loop, chaining as many tool turns as needed, until it reports the
//! goal achieved or blocked (a sentinel line) or a turn cap is reached. The
//! wasm-bindgen wrapper around `run` lives in the entrypoint cdylibs
//! (`crates/headless`, and any web+agent target that wants to expose it).

use serde_json::{json, Value};

use crate::agent::{build_request, compact, complete, exec, parse_tool_calls, system_prompt,
    tool_result_block};
use crate::github;

/// Sentinel lines the agent prints to end the run (see `goal_message`).
const ACHIEVED: &str = "GOAL_ACHIEVED";
const BLOCKED: &str = "GOAL_BLOCKED";

/// Pushed after a turn ends with no sentinel, to keep the agent moving.
const NUDGE: &str = "You ended your turn without signalling completion. If the goal is fully \
    achieved, reply with GOAL_ACHIEVED on its own line. If it is impossible or you are blocked, \
    reply GOAL_BLOCKED: <one-line reason>. Otherwise, keep working toward the goal.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Achieved,
    Blocked,
    MaxTurns,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Achieved => "achieved",
            Status::Blocked => "blocked",
            Status::MaxTurns => "max_turns",
        }
    }
}

/// Wrap the user's goal with the autonomous-operation contract.
fn goal_message(goal: &str) -> String {
    format!(
        "Autonomous goal — you have no interactive user; never ask for confirmation, just \
         act.\n\nGOAL:\n{goal}\n\nWork until the goal is fully achieved, chaining as many tool \
         calls as it takes. When it is done, output a final line containing exactly:\n\
         {ACHIEVED}\nIf it is impossible or you are blocked (missing permissions, bad input, \
         …), output a final line:\n{BLOCKED}: <one-line reason>\nDo not output either sentinel \
         until you are truly finished."
    )
}

/// Scan assistant text for a terminal sentinel leading its own line.
fn terminal_outcome(text: &str) -> Option<(Status, String)> {
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with(ACHIEVED) {
            return Some((Status::Achieved, "agent reported the goal achieved".to_string()));
        }
        if let Some(rest) = t.strip_prefix(BLOCKED) {
            let reason = rest.trim_start_matches(':').trim();
            let reason = if reason.is_empty() { "agent reported it is blocked" } else { reason };
            return Some((Status::Blocked, reason.to_string()));
        }
    }
    None
}

fn extract_text(resp: &Value) -> String {
    resp["content"]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter(|b| b["type"] == "text")
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

/// Drop a dangling assistant turn (unanswered tool_use / empty content) so
/// the next request stays valid after a mid-call `max_tokens` cut.
fn sanitize_tail(history: &mut Vec<Value>) {
    let Some(last) = history.last_mut() else { return };
    if last["role"] != "assistant" {
        return;
    }
    let Some(blocks) = last["content"].as_array_mut() else {
        history.pop();
        return;
    };
    blocks.retain(|b| b["type"] != "tool_use");
    let keeps_text = blocks.iter().any(|b| {
        b["type"] == "text" && b["text"].as_str().map(|t| !t.trim().is_empty()).unwrap_or(false)
    });
    if !keeps_text {
        history.pop();
    }
}

/// One summarization round; mirrors the in-app compaction (an overflow during
/// the summary itself falls back to a recover-from-VFS stub).
async fn compact_once(
    system: &str,
    history: &[Value],
    api_key: &str,
    base: Option<&str>,
) -> Result<Vec<Value>, String> {
    let body = compact::summary_request(system, history);
    match complete(api_key, base, body).await {
        Ok(resp) => {
            let summary = extract_text(&resp);
            if summary.trim().is_empty() {
                Err("compaction returned no summary".to_string())
            } else {
                Ok(compact::compacted_history(&summary))
            }
        }
        Err(e) if compact::is_overflow(&e) => Ok(compact::compacted_history(
            "(no summary — the context overflowed before one could be written; recover by \
             listing the shell files with ls / and re-reading them)",
        )),
        Err(e) => Err(e),
    }
}

/// End the run if the assistant signalled completion; otherwise append a
/// nudge and report "keep going".
fn resolve_or_nudge(history: &mut Vec<Value>, text: &str) -> Option<(Status, String)> {
    if let Some(o) = terminal_outcome(text) {
        return Some(o);
    }
    history.push(json!({"role": "user", "content": NUDGE}));
    None
}

/// Run the agent autonomously toward `goal`. `emit` receives JSON progress
/// events. Returns the terminal status and a one-line detail.
pub async fn run(
    goal: &str,
    token: Option<String>,
    api_key: &str,
    base: Option<&str>,
    max_turns: u32,
    emit: &dyn Fn(&Value),
) -> (Status, String) {
    crate::knowledge::seed();
    let login = github::current_user(&token).await.ok().map(|u| u.login);
    emit(&json!({"type": "start", "login": login}));
    let system = system_prompt(login.as_deref(), None, None);

    let mut history: Vec<Value> = vec![json!({"role": "user", "content": goal_message(goal)})];
    let mut ctx_tokens: u64 = 0;
    let mut pause_count: u32 = 0;
    let mut turn: u32 = 0;

    loop {
        if ctx_tokens >= compact::SOFT_CAP {
            emit(&json!({"type": "compact"}));
            match compact_once(&system, &history, api_key, base).await {
                Ok(h) => {
                    history = h;
                    ctx_tokens = 0;
                }
                Err(e) => {
                    emit(&json!({"type": "error", "message": e.clone()}));
                    return (Status::Blocked, e);
                }
            }
        }

        turn += 1;
        if turn > max_turns {
            emit(&json!({"type": "limit", "turns": max_turns}));
            return (Status::MaxTurns, format!("reached the {max_turns}-turn limit"));
        }
        emit(&json!({"type": "turn", "n": turn}));

        let resp = match complete(api_key, base, build_request(&system, &history)).await {
            Ok(r) => r,
            Err(e) if compact::is_overflow(&e) => {
                emit(&json!({"type": "compact"}));
                match compact_once(&system, &history, api_key, base).await {
                    Ok(h) => {
                        history = h;
                        ctx_tokens = 0;
                        continue;
                    }
                    Err(e2) => {
                        emit(&json!({"type": "error", "message": e2.clone()}));
                        return (Status::Blocked, e2);
                    }
                }
            }
            Err(e) => {
                emit(&json!({"type": "error", "message": e.clone()}));
                return (Status::Blocked, e);
            }
        };

        if let Some(t) = compact::context_tokens(&resp) {
            ctx_tokens = t;
        }
        let content = resp["content"].clone();
        let stop = resp["stop_reason"].as_str().unwrap_or("").to_string();
        if stop != "pause_turn" {
            pause_count = 0;
        }
        if content.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
            history.push(json!({"role": "assistant", "content": content.clone()}));
        }
        let mut text = String::new();
        if let Some(blocks) = content.as_array() {
            for b in blocks {
                if b["type"] == "text" {
                    if let Some(t) = b["text"].as_str().filter(|t| !t.trim().is_empty()) {
                        emit(&json!({"type": "text", "text": t}));
                        text.push_str(t);
                        text.push('\n');
                    }
                }
            }
        }

        match stop.as_str() {
            "pause_turn" => {
                pause_count += 1;
                if pause_count > 8 {
                    return (Status::Blocked, "the server kept pausing the turn".to_string());
                }
                // Resend the (assistant-tailed) history to resume.
            }
            "refusal" => {
                let cat = resp["stop_details"]["category"].as_str().unwrap_or("");
                let d = if cat.is_empty() {
                    "the model declined the request".to_string()
                } else {
                    format!("the model declined the request ({cat})")
                };
                return (Status::Blocked, d);
            }
            "max_tokens" => {
                sanitize_tail(&mut history);
                history.push(json!({"role": "user",
                    "content": "Your previous reply hit the token limit mid-step. Continue."}));
            }
            "tool_use" => {
                let calls = parse_tool_calls(&content);
                if calls.is_empty() {
                    if let Some(o) = resolve_or_nudge(&mut history, &text) {
                        return o;
                    }
                } else {
                    let mut blocks = Vec::with_capacity(calls.len());
                    for c in &calls {
                        emit(&json!({"type": "tool", "label": c.label()}));
                        let (out, ok) = exec(&token, c).await;
                        emit(&json!({"type": "tool_done", "label": c.label(), "ok": ok}));
                        blocks.push(tool_result_block(c.id(), &out, ok));
                    }
                    history.push(json!({"role": "user", "content": blocks}));
                }
            }
            _ => {
                // end_turn (or unknown): done unless still mid-task.
                if let Some(o) = resolve_or_nudge(&mut history, &text) {
                    return o;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{terminal_outcome, Status};

    #[test]
    fn detects_achieved_and_blocked_sentinels() {
        assert!(matches!(
            terminal_outcome("did the thing\nGOAL_ACHIEVED"),
            Some((Status::Achieved, _))
        ));
        match terminal_outcome("tried\nGOAL_BLOCKED: missing write scope") {
            Some((Status::Blocked, r)) => assert_eq!(r, "missing write scope"),
            other => panic!("expected blocked, got {other:?}"),
        }
        assert!(terminal_outcome("still working on it").is_none());
        // Inline mentions don't count — the sentinel must lead its own line.
        assert!(terminal_outcome("will print GOAL_ACHIEVED when finished").is_none());
    }

    #[test]
    fn blocked_without_reason_still_resolves() {
        assert!(matches!(terminal_outcome("GOAL_BLOCKED"), Some((Status::Blocked, _))));
    }
}
