//! Paste routing and click application (clicks arrive already resolved by
//! the px view's hit-testing).

use crate::ui::input::{Key, Mods};

use super::file_msgs::rehighlight;
use super::{App, Click, Overlay, RepoFocus, Route, Tab};

impl App {
    pub(super) fn on_paste(&mut self, text: String) -> bool {
        match &mut self.overlay {
            Some(Overlay::Commit(input)) | Some(Overlay::OpenRepo(input)) => {
                input.insert(&text.replace('\n', " "));
                return true;
            }
            Some(Overlay::FileSearch { input, sel }) => {
                input.insert(&text.replace('\n', " "));
                *sel = 0;
                return true;
            }
            Some(Overlay::CodeSearch { input, .. }) => {
                input.insert(&text.replace('\n', " "));
                return true;
            }
            Some(_) => return false,
            None => {}
        }
        match self.route {
            Route::Auth => {
                // Same lock as typed keys: no mutating the token while a
                // validation request is in flight.
                if self.auth_busy {
                    return false;
                }
                self.token_input.insert(text.trim());
                true
            }
            Route::Agent => {
                if self.anthropic_key.is_none() {
                    if self.agent.url_focused {
                        self.agent.url_input.insert(text.trim());
                    } else {
                        self.agent.key_input.insert(text.trim());
                    }
                } else {
                    self.agent.input.insert(&text.replace('\n', " "));
                }
                true
            }
            Route::Repos if self.filter_active => {
                self.filter.insert(&text.replace('\n', " "));
                // Pasted filter text resets selection like typed chars do.
                self.repo_sel = 0;
                self.repo_scroll = 0;
                true
            }
            Route::Repo => {
                let lay = self.layout;
                if self.in_editor() {
                    if let Some(rv) = &mut self.rv {
                        if let Some(f) = &mut rv.file {
                            f.editor.insert_text(&text);
                            f.editor.ensure_visible(
                                lay.content_text.h.max(1) as usize,
                                lay.content_text.w.max(1) as usize,
                            );
                            rehighlight(f);
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Apply a click resolved by the px view's hit-testing.
    pub fn perform_click(&mut self, click: Click) {
        self.toast = None;
        self.dirty = true;
        match click {
            Click::Repo(i) => {
                if self.repo_sel == i {
                    self.repos_key(Key::Enter, Mods::NONE);
                } else {
                    self.repo_sel = i;
                }
            }
            Click::TreeRow(i) => {
                let Some(rv) = &mut self.rv else { return };
                rv.focus = RepoFocus::Tree;
                if rv.tree_sel == i {
                    self.activate_tree_row(false);
                } else {
                    rv.tree_sel = i;
                }
            }
            Click::Tab(t) => {
                let Some(rv) = &mut self.rv else { return };
                rv.tab = t;
                if t == Tab::Actions && matches!(rv.runs, super::Loadable::Idle) {
                    self.load_runs();
                }
            }
            Click::BranchBtn => {
                if self.overlay.is_none() {
                    self.code_key(Key::Char('b'), Mods::NONE);
                }
            }
            Click::Run(i) => {
                let Some(rv) = &mut self.rv else { return };
                if rv.runs_sel == i {
                    self.actions_key(Key::Enter, Mods::NONE);
                } else {
                    rv.runs_sel = i;
                }
            }
            Click::EditorPos { row, cell_x } => {
                let Some(rv) = &mut self.rv else { return };
                rv.focus = RepoFocus::Content;
                if let Some(f) = &mut rv.file {
                    let row = row.min(f.editor.line_count() - 1);
                    let col = f.editor.x_to_col(row, cell_x);
                    f.editor.move_to((row, col), false);
                }
            }
            Click::OverlayItem(i) => {
                let sel = match &mut self.overlay {
                    Some(Overlay::BranchPick { sel, .. }) => Some(sel),
                    Some(Overlay::FileSearch { sel, .. }) => Some(sel),
                    Some(Overlay::CodeSearch { sel, .. }) => Some(sel),
                    _ => None,
                };
                if let Some(sel) = sel {
                    if *sel == i {
                        self.overlay_key(Key::Enter, Mods::NONE);
                    } else {
                        *sel = i;
                    }
                }
            }
            Click::EditBtn => self.begin_edit(),
            Click::CommitBtn => self.begin_commit(),
            Click::AgentClear => self.agent_clear(),
            Click::AgentResetKey => {
                if self.agent.busy {
                    self.agent_cancel();
                }
                crate::agent::clear_key();
                self.anthropic_key = None;
                self.agent.key_input.clear();
                // Pre-fill the endpoint so changing only the key keeps it.
                self.agent.url_input.clear();
                if let Some(u) = &self.anthropic_url {
                    self.agent.url_input.insert(u);
                }
                self.agent.url_focused = false;
            }
            Click::SortCycle => self.cycle_sort(),
            Click::SortDir => {
                self.sort_asc = !self.sort_asc;
                self.repo_sel = 0;
            }
            Click::ToggleForks => {
                self.hide_forks = !self.hide_forks;
                self.repo_sel = 0;
            }
            Click::ToggleArchived => {
                self.hide_archived = !self.hide_archived;
                self.repo_sel = 0;
            }
        }
    }
}
