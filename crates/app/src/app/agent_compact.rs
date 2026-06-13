//! Compaction driver: when the context window fills (or the API reports
//! an overflow), divert the loop into a summarization request, replace
//! the history with the model's handoff summary, and resume the task.
//! History is only replaced on success, so every failure path leaves a
//! valid, sendable history behind.

use serde_json::Value;

use super::chat::AgentItem;
use super::{App, Msg};
use crate::agent::compact;

impl App {
    /// Divert this turn into compaction when the context is nearly full.
    /// True means diverted: the caller must not send its own request.
    pub(super) fn maybe_compact(&mut self) -> bool {
        if self.agent.compacting.is_some() || self.agent.ctx_tokens < compact::SOFT_CAP {
            return false;
        }
        self.start_compaction();
        true
    }

    pub(super) fn start_compaction(&mut self) {
        let Some(key) = self.anthropic_key.clone() else {
            self.agent.busy = false;
            self.agent.push(AgentItem::Error("no API key configured".into()));
            return;
        };
        self.agent.push(AgentItem::Tool { label: "compact context".into(), done: None });
        self.agent.compacting = Some(self.agent.transcript.len() - 1);
        let body =
            compact::summary_request(&self.agent_model, &self.agent_system(), &self.agent.history);
        let base = self.anthropic_url.clone();
        let gen = self.agent.gen;
        crate::spawn_msg(async move {
            Msg::AgentResponse {
                gen,
                result: crate::agent::complete(&key, base.as_deref(), body).await,
            }
        });
    }

    pub(super) fn on_compact_response(&mut self, result: Result<Value, String>) {
        let item = self.agent.compacting.take();
        match result {
            Ok(resp) => {
                let summary = resp["content"]
                    .as_array()
                    .map(|blocks| {
                        blocks
                            .iter()
                            .filter(|b| b["type"] == "text")
                            .filter_map(|b| b["text"].as_str())
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                if summary.trim().is_empty() {
                    self.agent.busy = false;
                    self.mark_compact_item(item, false);
                    self.agent.push(AgentItem::Error("compaction returned no summary".into()));
                    return;
                }
                self.agent.history = compact::compacted_history(&summary);
                self.agent.ctx_tokens = 0;
                self.mark_compact_item(item, true);
                self.agent_turn();
            }
            // Even the trimmed summary request overflowed — reset hard; the
            // VFS still holds everything the agent needs to recover.
            Err(e) if compact::is_overflow(&e) => {
                self.agent.history = compact::compacted_history(
                    "(no summary — the context overflowed before one could be written; \
                     recover by listing the shell files with ls / and re-reading them)",
                );
                self.agent.ctx_tokens = 0;
                self.mark_compact_item(item, false);
                self.agent_turn();
            }
            Err(e) => {
                self.agent.busy = false;
                self.mark_compact_item(item, false);
                self.agent.push(AgentItem::Error(e));
            }
        }
    }

    fn mark_compact_item(&mut self, item: Option<usize>, ok: bool) {
        if let Some(AgentItem::Tool { done, .. }) =
            item.and_then(|i| self.agent.transcript.get_mut(i))
        {
            *done = Some(ok);
            self.agent.rev += 1;
        }
    }
}
