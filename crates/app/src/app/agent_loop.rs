//! The agent loop proper: firing Messages API turns, dispatching on
//! stop_reason, and executing tool batches (with cancel checks between
//! sequential executions).

use serde_json::Value;

use super::agent_history::LIVE_GEN;
use super::chat::AgentItem;
use super::{App, Msg};

impl App {
    /// Fire one Messages API request for the current history.
    pub(super) fn agent_turn(&mut self) {
        let Some(key) = self.anthropic_key.clone() else {
            // Latent path (callers check the key) — but silently returning
            // here would leave busy=true forever.
            self.agent.busy = false;
            self.agent.push(AgentItem::Error("no API key configured".into()));
            return;
        };
        if self.maybe_compact() {
            return;
        }
        let body =
            crate::agent::build_request(&self.agent_model, &self.agent_system(), &self.agent.history);
        let base = self.anthropic_url.clone();
        let gen = self.agent.gen;
        crate::spawn_msg(async move {
            Msg::AgentResponse {
                gen,
                result: crate::agent::complete(&key, base.as_deref(), body).await,
            }
        });
    }

    /// System prompt for the current login/repo/file context (shared with
    /// the compaction request so the cached prefix stays identical).
    pub(super) fn agent_system(&self) -> String {
        let repo_ctx = self
            .rv
            .as_ref()
            .map(|rv| (rv.repo.full_name.clone(), rv.branch.clone()));
        let file = self
            .rv
            .as_ref()
            .and_then(|rv| rv.file.as_ref())
            .map(|f| f.path.clone());
        crate::agent::system_prompt(
            self.login.as_deref(),
            repo_ctx.as_ref().map(|(r, b)| (r.as_str(), b.as_str())),
            file.as_deref(),
        )
    }

    pub(super) fn on_agent_response_msg(&mut self, gen: u64, result: Result<Value, String>) {
        if gen != self.agent.gen || !self.agent.busy {
            return; // cancelled or superseded
        }
        if self.agent.compacting.is_some() {
            self.on_compact_response(result);
            return;
        }
        self.on_agent_response(result);
    }

    pub(super) fn on_agent_tools_done(&mut self, gen: u64, results: Vec<(Value, bool)>) {
        if gen != self.agent.gen || !self.agent.busy {
            return;
        }
        for (i, &idx) in self.agent.pending.iter().enumerate() {
            if let Some(AgentItem::Tool { done, .. }) = self.agent.transcript.get_mut(idx) {
                *done = results.get(i).map(|(_, ok)| *ok);
            }
        }
        self.agent.pending.clear();
        self.agent.rev += 1;
        let blocks: Vec<Value> = results.into_iter().map(|(b, _)| b).collect();
        self.agent
            .history
            .push(serde_json::json!({"role": "user", "content": blocks}));
        self.agent_turn();
    }

    fn on_agent_response(&mut self, result: Result<Value, String>) {
        let resp = match result {
            Ok(r) => r,
            Err(e) => {
                // Context overflow is recoverable: summarize and retry.
                if crate::agent::compact::is_overflow(&e) {
                    self.start_compaction();
                    return;
                }
                self.agent.busy = false;
                // A pause_turn resend that failed leaves a trailing
                // assistant message; make the history sendable again.
                self.sanitize_history_tail();
                self.agent.push(AgentItem::Error(e));
                return;
            }
        };
        if let Some(t) = crate::agent::compact::context_tokens(&resp) {
            self.agent.ctx_tokens = t;
        }
        let content = resp["content"].clone();
        let stop = resp["stop_reason"].as_str().unwrap_or("").to_string();
        if stop != "pause_turn" {
            self.agent.pause_count = 0;
        }
        // An empty content array (pre-output refusal) must not enter the
        // history — the API rejects empty assistant turns on later sends.
        let has_content = content.as_array().map(|a| !a.is_empty()).unwrap_or(false);
        if has_content {
            self.agent
                .history
                .push(serde_json::json!({"role": "assistant", "content": content}));
        }
        if let Some(blocks) = content.as_array() {
            for b in blocks {
                if b["type"] == "text" {
                    if let Some(t) = b["text"].as_str() {
                        if !t.trim().is_empty() {
                            self.agent.push(AgentItem::Text(t.to_string()));
                        }
                    }
                }
            }
        }
        match stop.as_str() {
            "tool_use" => {
                let calls = crate::agent::parse_tool_calls(&content);
                if calls.is_empty() {
                    self.agent.busy = false;
                    self.sanitize_history_tail();
                    return;
                }
                self.agent.pending.clear();
                for c in &calls {
                    self.agent.push(AgentItem::Tool { label: c.label(), done: None });
                    self.agent.pending.push(self.agent.transcript.len() - 1);
                }
                let token = self.token.clone();
                let gen = self.agent.gen;
                crate::spawn_msg(async move {
                    let mut results = Vec::with_capacity(calls.len());
                    for c in &calls {
                        // A cancel orphans the results; it must also stop
                        // the remaining (possibly mutating) executions.
                        if LIVE_GEN.with(|g| g.get()) != gen {
                            break;
                        }
                        let (text, ok) = crate::agent::exec(&token, c).await;
                        results.push((crate::agent::tool_result_block(c.id(), &text, ok), ok));
                    }
                    Msg::AgentToolsDone { gen, results }
                });
            }
            // Server-side pause (defensive — no server tools configured):
            // re-send and the API resumes where it left off.
            "pause_turn" => {
                self.agent.pause_count += 1;
                if self.agent.pause_count > 8 {
                    self.agent.busy = false;
                    self.agent.pause_count = 0;
                    self.sanitize_history_tail();
                    self.agent.push(AgentItem::Error("server kept pausing the turn".into()));
                } else {
                    self.agent_turn();
                }
            }
            "refusal" => {
                self.agent.busy = false;
                self.sanitize_history_tail();
                let cat = resp["stop_details"]["category"].as_str().unwrap_or("");
                let msg = if cat.is_empty() {
                    "request declined by the model".to_string()
                } else {
                    format!("request declined by the model ({})", cat)
                };
                self.agent.push(AgentItem::Error(msg));
            }
            "max_tokens" => {
                self.agent.busy = false;
                // The cut can land mid-tool-call; without sanitizing, every
                // later send would 400 on the unanswered tool_use.
                self.sanitize_history_tail();
                self.agent
                    .push(AgentItem::Error("response hit the token limit — say 'continue'".into()));
            }
            _ => {
                self.agent.busy = false; // end_turn
                self.sanitize_history_tail();
            }
        }
    }
}
