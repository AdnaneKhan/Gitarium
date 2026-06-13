//! Agent chat state (transcript + verbatim API history) and the Agent
//! route's key handling.

use crate::ui::input::{Key, Mods};
use crate::ui::lineinput::LineInput;

use super::agent_history::{push_user_text, LIVE_GEN};
use super::{App, Route};

/// One entry in the agent window's transcript.
pub enum AgentItem {
    User(String),
    Text(String),
    /// A github_api invocation; `done` is None while in flight.
    Tool { label: String, done: Option<bool> },
    Error(String),
}

pub struct AgentChat {
    pub key_input: LineInput,
    pub url_input: LineInput,
    /// Which field the key panel is editing (false = API key, true = URL).
    pub url_focused: bool,
    pub input: LineInput,
    pub transcript: Vec<AgentItem>,
    /// Verbatim Messages API history (assistant content blocks are echoed
    /// back unchanged so thinking/tool_use pairing stays valid).
    pub history: Vec<serde_json::Value>,
    pub busy: bool,
    /// Bumped to invalidate in-flight futures (cancel / clear).
    pub gen: u64,
    /// Bumped on every transcript change; the view uses it to re-stick the
    /// scroll position to the bottom.
    pub rev: u64,
    /// Transcript indices of Tool items awaiting results.
    pub pending: Vec<usize>,
    /// Consecutive `pause_turn` resends, capped so a misbehaving server
    /// can't loop the turn forever.
    pub pause_count: u32,
    /// Context tokens consumed by the last completed turn (from the API
    /// usage block); drives proactive compaction.
    pub ctx_tokens: u64,
    /// Transcript index of the in-flight "compact context" item; Some
    /// doubles as the "a compaction request is in flight" flag.
    pub compacting: Option<usize>,
}

impl AgentChat {
    pub(super) fn new() -> Self {
        AgentChat {
            key_input: LineInput::new(true),
            url_input: LineInput::new(false),
            url_focused: false,
            input: LineInput::new(false),
            transcript: Vec::new(),
            history: Vec::new(),
            busy: false,
            gen: 0,
            rev: 0,
            pending: Vec::new(),
            pause_count: 0,
            ctx_tokens: 0,
            compacting: None,
        }
    }

    pub(super) fn push(&mut self, item: AgentItem) {
        self.transcript.push(item);
        self.rev += 1;
    }
}

impl App {
    pub(super) fn open_agent(&mut self) {
        self.route = Route::Agent;
    }

    pub(super) fn leave_agent(&mut self) {
        self.route = if self.rv.is_some() { Route::Repo } else { Route::Repos };
    }

    /// Open the model picker and fetch the provider's model list.
    pub(super) fn open_model_pick(&mut self) {
        let Some(key) = self.anthropic_key.clone() else {
            self.toast = Some(("set an API key first".into(), true));
            return;
        };
        self.overlay = Some(super::Overlay::ModelPick { models: super::Loadable::Loading, sel: 0 });
        let base = self.anthropic_url.clone();
        crate::spawn_msg(async move {
            let result = crate::agent::list_models(&key, base.as_deref()).await;
            super::Msg::ModelsListed { result }
        });
    }

    pub(super) fn on_models_listed(
        &mut self,
        result: Result<Vec<crate::agent::ModelInfo>, String>,
    ) {
        let cur = self.agent_model.clone();
        if let Some(super::Overlay::ModelPick { models, sel }) = &mut self.overlay {
            match result {
                Ok(list) => {
                    *sel = list.iter().position(|m| m.id == cur).unwrap_or(0);
                    *models = super::Loadable::Ready(list);
                }
                Err(e) => *models = super::Loadable::Failed(e),
            }
        }
    }

    /// Persist + apply the chosen model, then close the picker.
    pub(super) fn select_model(&mut self, id: String) {
        crate::agent::save_model(&id);
        self.agent_model = id;
        self.overlay = None;
        self.toast = Some(("model updated".into(), false));
    }

    pub(super) fn agent_send(&mut self) {
        let text = self.agent.input.text.trim().to_string();
        if text.is_empty() || self.agent.busy {
            return;
        }
        self.agent.input.clear();
        self.agent.push(AgentItem::User(text.clone()));
        push_user_text(&mut self.agent.history, &text);
        self.agent.busy = true;
        self.agent.gen += 1;
        LIVE_GEN.with(|g| g.set(self.agent.gen));
        self.agent.pause_count = 0;
        self.agent_turn();
    }

    pub(super) fn agent_key(&mut self, key: Key, mods: Mods) -> bool {
        // No API key yet: the window shows the key/endpoint prompt.
        if self.anthropic_key.is_none() {
            return match key {
                Key::Esc => {
                    self.leave_agent();
                    true
                }
                Key::Tab | Key::BackTab | Key::Up | Key::Down => {
                    self.agent.url_focused = !self.agent.url_focused;
                    true
                }
                Key::Enter => {
                    let k = self.agent.key_input.text.trim().to_string();
                    if k.is_empty() {
                        return true;
                    }
                    let url = crate::agent::normalize_base(&self.agent.url_input.text);
                    match &url {
                        Some(u) => crate::agent::save_url(u),
                        None => crate::agent::clear_url(),
                    }
                    self.anthropic_url = url;
                    crate::agent::save_key(&k);
                    self.anthropic_key = Some(k);
                    self.agent.key_input.clear();
                    self.agent.url_input.clear();
                    self.agent.url_focused = false;
                    true
                }
                k => {
                    if self.agent.url_focused {
                        self.agent.url_input.handle_key(&k, mods)
                    } else {
                        self.agent.key_input.handle_key(&k, mods)
                    }
                }
            };
        }
        match key {
            Key::Esc => {
                if self.agent.busy {
                    self.agent_cancel();
                } else {
                    self.leave_agent();
                }
                true
            }
            Key::Enter => {
                self.agent_send();
                true
            }
            k => self.agent.input.handle_key(&k, mods),
        }
    }
}
