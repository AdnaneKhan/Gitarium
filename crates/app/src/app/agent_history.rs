//! Keeping the Messages-API history an invariant, not a log: every path
//! that ends a turn must leave `history` in a shape the API accepts
//! (alternation, tool_use answered, non-empty content).

use std::cell::Cell;

use super::chat::AgentItem;
use super::App;

thread_local! {
    /// Mirror of `AgentChat::gen` readable from detached futures: a cancel
    /// bumps it, and the sequential tool batch re-checks it between
    /// executions so mutating calls stop instead of outliving the cancel.
    pub(super) static LIVE_GEN: Cell<u64> = const { Cell::new(0) };
}

/// Append user text to the Messages history, merging into a trailing user
/// message when one exists (cancel/error paths can leave the history ending
/// on a tool_result turn; consecutive user messages are rejected by the
/// API, while one message with tool_results followed by text is valid).
pub(super) fn push_user_text(history: &mut Vec<serde_json::Value>, text: &str) {
    use serde_json::{json, Value};
    if let Some(last) = history.last_mut() {
        if last["role"] == "user" {
            let block = json!({"type": "text", "text": text});
            match &mut last["content"] {
                Value::String(s) => {
                    let prev = std::mem::take(s);
                    last["content"] = json!([{"type": "text", "text": prev}, block]);
                }
                Value::Array(a) => a.push(block),
                _ => {}
            }
            return;
        }
    }
    history.push(json!({"role": "user", "content": text}));
}

impl App {
    pub(super) fn agent_cancel(&mut self) {
        self.agent.gen += 1; // orphan any in-flight future
        LIVE_GEN.with(|g| g.set(self.agent.gen));
        self.agent.busy = false;
        for &i in &self.agent.pending {
            if let Some(AgentItem::Tool { done, .. }) = self.agent.transcript.get_mut(i) {
                *done = Some(false);
            }
        }
        self.agent.pending.clear();
        // A cancelled compaction must not capture the next response.
        if let Some(AgentItem::Tool { done, .. }) =
            self.agent.compacting.take().and_then(|i| self.agent.transcript.get_mut(i))
        {
            *done = Some(false);
        }
        self.sanitize_history_tail();
        self.agent.push(AgentItem::Error("cancelled".into()));
    }

    pub(super) fn agent_clear(&mut self) {
        if self.agent.busy {
            self.agent_cancel();
        }
        self.agent.transcript.clear();
        self.agent.history.clear();
        self.agent.pending.clear();
        self.agent.ctx_tokens = 0;
        crate::agent::clear_store();
        self.agent.rev += 1;
    }

    /// Leave `history` in a shape the Messages API accepts on the next
    /// send: the final message must not be an assistant turn with
    /// unanswered tool_use blocks, and content must stay non-empty. Text
    /// the model produced is kept (the transcript shows it, so the model
    /// should remember it); tool_use blocks that will never get results
    /// are stripped. Every terminal path (cancel, error, refusal,
    /// max_tokens) funnels through here.
    pub(super) fn sanitize_history_tail(&mut self) {
        let Some(last) = self.agent.history.last_mut() else { return };
        if last["role"] != "assistant" {
            return;
        }
        let Some(blocks) = last["content"].as_array_mut() else {
            self.agent.history.pop();
            return;
        };
        blocks.retain(|b| b["type"] != "tool_use");
        // Thinking-only (or empty) remainders are dropped whole: the API
        // rejects assistant turns without displayable content.
        let keeps_text = blocks.iter().any(|b| {
            b["type"] == "text"
                && b["text"].as_str().map(|t| !t.trim().is_empty()).unwrap_or(false)
        });
        if !keeps_text {
            self.agent.history.pop();
        }
    }
}
