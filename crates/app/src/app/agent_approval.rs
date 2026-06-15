//! Manual-approval gate for the interactive agent's mutating API calls.
//!
//! Any non-GET `github_api` call pauses the loop for explicit human approval
//! (a modal) before it runs — unless YOLO mode (auto-approve) is on, which is
//! itself gated behind a risk warning. The headless agent never reaches here:
//! it has its own UI-free loop and runs autonomously.

use serde_json::json;

use crate::agent::{exec, parse_tool_calls, tool_result_block, ToolCall};

use super::agent_history::LIVE_GEN;
use super::chat::AgentItem;
use super::{App, Msg, Overlay};

/// One bullet per write call for the approval modal (the read-only calls in
/// the same turn aren't listed — only what actually changes state matters).
pub(super) fn write_summary(calls: &[ToolCall]) -> String {
    let mut s = String::new();
    for c in calls.iter().filter(|c| c.is_mutating()) {
        s.push_str("• ");
        s.push_str(&c.label());
        s.push('\n');
    }
    s
}

impl App {
    /// Push transcript Tool rows for `calls` and spawn their sequential
    /// execution; the assistant turn is already in `history`. Shared by the
    /// YOLO path and the approve path so both run identically.
    pub(super) fn run_tool_batch(&mut self, calls: Vec<ToolCall>) {
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
                // A cancel orphans the results; it must also stop the
                // remaining (possibly mutating) executions.
                if LIVE_GEN.with(|g| g.get()) != gen {
                    break;
                }
                let (text, ok) = exec(&token, c).await;
                results.push((tool_result_block(c.id(), &text, ok), ok));
            }
            Msg::AgentToolsDone { gen, results }
        });
    }

    /// Approve the pending mutating turn: re-parse its tool calls and run them.
    fn approve_agent_tools(&mut self) {
        let Some(Overlay::AgentApproval { content, .. }) = self.overlay.take() else { return };
        self.agent.busy = true;
        self.run_tool_batch(parse_tool_calls(&content));
    }

    /// Deny the pending mutating turn. The Messages API requires every
    /// tool_use to be answered, so each call gets a refusal tool_result; the
    /// loop then continues so the model can acknowledge and adjust.
    fn deny_agent_tools(&mut self) {
        let Some(Overlay::AgentApproval { content, .. }) = self.overlay.take() else { return };
        let calls = parse_tool_calls(&content);
        let mut blocks = Vec::with_capacity(calls.len());
        for c in &calls {
            self.agent.push(AgentItem::Tool { label: c.label(), done: Some(false) });
            blocks.push(tool_result_block(
                c.id(),
                "The user declined to approve this action. Do not retry it; \
                 stop and explain, or ask how they would like to proceed.",
                false,
            ));
        }
        self.agent.history.push(json!({"role": "user", "content": blocks}));
        self.agent.busy = true;
        self.agent_turn();
    }

    /// YOLO chip: enabling needs the risk modal first; disabling is immediate.
    pub(super) fn toggle_yolo(&mut self) {
        if self.yolo {
            self.yolo = false;
            self.toast = Some(("auto-approve OFF — writes ask for approval".into(), false));
        } else {
            self.overlay = Some(Overlay::YoloWarn);
        }
    }

    /// Confirm the risk warning and turn auto-approve on.
    fn enable_yolo(&mut self) {
        self.yolo = true;
        self.overlay = None;
        self.toast = Some(("YOLO ON — mutating API calls run without asking".into(), true));
    }

    pub(super) fn agent_approval_key(
        &mut self,
        key: crate::ui::input::Key,
        mods: crate::ui::input::Mods,
    ) -> bool {
        use crate::ui::input::Key;
        match key {
            Key::Enter | Key::Char('y') | Key::Char('Y') if super::keys::plain(mods) => {
                self.approve_agent_tools();
                true
            }
            Key::Esc | Key::Char('n') | Key::Char('N') if super::keys::plain(mods) => {
                self.deny_agent_tools();
                true
            }
            // Modal: ignore any other key so a stray press can't run a write.
            _ => true,
        }
    }

    pub(super) fn yolo_warn_key(
        &mut self,
        key: crate::ui::input::Key,
        mods: crate::ui::input::Mods,
    ) -> bool {
        use crate::ui::input::Key;
        match key {
            Key::Enter | Key::Char('y') | Key::Char('Y') if super::keys::plain(mods) => {
                self.enable_yolo();
                true
            }
            Key::Esc | Key::Char('n') | Key::Char('N') if super::keys::plain(mods) => {
                self.overlay = None;
                true
            }
            _ => true,
        }
    }
}
