//! Paste routing and click application (clicks arrive already resolved by
//! the px view's hit-testing).

use crate::ui::input::{Key, Mods};

use super::file_msgs::rehighlight;
use super::{App, Click, Overlay, RepoFocus, Route, Tab};

impl App {
    pub(super) fn on_paste(&mut self, text: String) -> bool {
        match &mut self.overlay {
            Some(Overlay::Commit(form)) => {
                // The commit message may span lines; override fields are
                // single-line, so collapse newlines there.
                if form.field == 0 {
                    form.message.insert(&text);
                } else {
                    form.focused().insert(&text.replace('\n', " "));
                }
                return true;
            }
            Some(Overlay::NewFile(input)) | Some(Overlay::OpenRepo(input)) => {
                input.insert(&text.replace('\n', " "));
                return true;
            }
            Some(Overlay::NewBranch { name, .. }) => {
                name.insert(&text.replace('\n', " "));
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
            Click::Tab(t) => self.switch_tab(t),
            Click::IssueRow(i) => {
                let Some(rv) = &mut self.rv else { return };
                let is_pulls = matches!(rv.tab, Tab::Pulls);
                let sel = if is_pulls { rv.pulls_sel } else { rv.issues_sel };
                if sel == i {
                    if is_pulls {
                        self.open_pull_detail(i);
                    } else {
                        self.open_issue_detail(i);
                    }
                } else if is_pulls {
                    rv.pulls_sel = i;
                } else {
                    rv.issues_sel = i;
                }
            }
            Click::DetailSearchOpen => self.open_detail_search(),
            Click::DetailSearchClose => self.close_detail_search(),
            Click::DetailSearchPrev => self.detail_search_step(-1),
            Click::DetailSearchNext => self.detail_search_step(1),
            Click::Approve => self.approve_pr(),
            Click::Merge => self.merge_pr(),
            Click::MergeMethodCycle => {
                if let Some(d) = self.rv.as_mut().and_then(|rv| rv.detail.as_mut()) {
                    d.merge_method = d.merge_method.next();
                }
            }
            Click::DetailBack => self.close_detail(),
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
            Click::JobRow(i) => {
                let id = self
                    .rv
                    .as_ref()
                    .and_then(|rv| rv.jobs.as_ref())
                    .and_then(|(_, l)| l.ready())
                    .and_then(|jobs| jobs.get(i))
                    .map(|j| j.id);
                if let Some(id) = id {
                    self.open_job_logs(id);
                }
            }
            Click::JobLogBack => self.close_job_logs(),
            Click::LogSearchOpen => self.open_log_search(),
            Click::LogSearchClose => self.close_log_search(),
            Click::LogSearchPrev => self.log_match_step(-1),
            Click::LogSearchNext => self.log_match_step(1),
            // Handled in the view layer (needs DOM); kept here for exhaustiveness.
            Click::DownloadLog => {}
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
                    Some(Overlay::ModelPick { sel, .. }) => Some(sel),
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
            Click::StageBtn => self.stage_file_action(),
            Click::NewFileBtn => self.begin_new_file(),
            Click::CommitBtn => self.begin_commit(),
            Click::CommitCycleTarget => {
                if let Some(Overlay::Commit(form)) = &mut self.overlay {
                    form.target = form.target.next();
                    form.field = super::CommitForm::TARGET_FIELD;
                }
            }
            Click::NewBranchBtn => self.open_new_branch_modal(),
            Click::CycleBranchBase => {
                let n = self
                    .rv
                    .as_ref()
                    .and_then(|rv| rv.branches.ready().map(|b| b.len()))
                    .unwrap_or(0);
                if let Some(Overlay::NewBranch { base, .. }) = &mut self.overlay {
                    if n > 0 {
                        *base = (*base + 1) % n;
                    }
                }
            }
            // Handled in the px view layer (opens a browser tab) before it
            // ever reaches here; this arm only keeps the match exhaustive.
            Click::OpenUrl(_) => {}
            Click::ModelPickBtn => self.open_model_pick(),
            Click::AgentYolo => self.toggle_yolo(),
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
            Click::SettingsNav(i) => {
                if let Some(&sec) = super::visible_sections(self.is_admin()).get(i) {
                    self.switch_settings_section(sec);
                }
            }
            Click::SettingsRow(i) => {
                if let Some(rv) = &mut self.rv {
                    rv.settings.list_sel = i;
                }
            }
            Click::SettingsAdd => {
                if let Some(s) = self.rv.as_ref().map(|rv| rv.settings.section) {
                    self.open_section_create(s);
                }
            }
            Click::SettingsEdit => {
                if let Some(s) = self.rv.as_ref().map(|rv| rv.settings.section) {
                    self.open_section_edit(s);
                }
            }
            Click::SettingsDelete => {
                if let Some(s) = self.rv.as_ref().map(|rv| rv.settings.section) {
                    self.request_section_delete(s);
                }
            }
            Click::SettingsCycleChip => self.settings_cycle_chip(),
            Click::SettingsFocusField(i) => self.settings_focus_field(i),
            Click::SettingsCycleContentType => self.settings_cycle_content_type(),
            Click::SettingsToggleEvent(i) => self.settings_toggle_event(i),
            Click::SettingsArchiveRepo => self.request_archive_repo(),
            Click::SettingsDeleteRepo => self.request_delete_repo(),
        }
    }
}
