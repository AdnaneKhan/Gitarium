//! Overlay key dispatch plus the simple overlays (commit message, open
//! repo, branch picker, confirm). The two search palettes live in search.rs.

use crate::github;
use crate::ui::input::{Key, Mods};

use super::keys::plain;
use super::{App, CommitForm, ConfirmAction, Msg, Overlay, Route};

impl App {
    pub(super) fn overlay_key(&mut self, key: Key, mods: Mods) -> bool {
        match &self.overlay {
            None => false,
            Some(Overlay::Help) => {
                self.overlay = None;
                true
            }
            Some(Overlay::Commit(_)) => self.commit_overlay_key(key, mods),
            Some(Overlay::NewFile(_)) => self.newfile_overlay_key(key, mods),
            Some(Overlay::NewBranch { .. }) => self.new_branch_key(key, mods),
            Some(Overlay::OpenRepo(_)) => self.open_repo_overlay_key(key, mods),
            Some(Overlay::BranchPick { .. }) => self.branch_pick_key(key),
            Some(Overlay::ModelPick { .. }) => self.model_pick_key(key),
            Some(Overlay::Confirm { .. }) => self.confirm_key(key, mods),
            Some(Overlay::FileSearch { .. }) => self.file_search_key(key, mods),
            Some(Overlay::CodeSearch { .. }) => self.code_search_key(key, mods),
            Some(Overlay::SettingsForm(_)) => self.settings_form_key(key, mods),
        }
    }

    fn commit_overlay_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(Overlay::Commit(form)) = &mut self.overlay else { return false };
        let fields = CommitForm::FIELDS;
        let on_target = form.field == CommitForm::TARGET_FIELD;
        match key {
            Key::Esc => {
                self.overlay = None;
                true
            }
            // Tab / Up / Down move between the controls.
            Key::Tab | Key::Down => {
                form.field = (form.field + 1) % fields;
                true
            }
            Key::BackTab | Key::Up => {
                form.field = (form.field + fields - 1) % fields;
                true
            }
            // The target chip cycles current → new branch → new tag.
            Key::Right | Key::Char(' ') if on_target => {
                form.target = form.target.next();
                true
            }
            Key::Left if on_target => {
                form.target = form.target.prev();
                true
            }
            Key::Enter => {
                let msg = form.message.text.trim().to_string();
                if msg.is_empty() {
                    self.toast = Some(("commit message required".into(), false));
                    return true;
                }
                let id = form.identity();
                let target = form.target;
                let name = form.new_ref.text.trim().to_string();
                self.overlay = None;
                self.commit_staged(msg, id, target, name);
                true
            }
            // The chip isn't a text input — swallow any other key on it.
            _ if on_target => true,
            k => form.focused().handle_key(&k, mods),
        }
    }

    fn model_pick_key(&mut self, key: Key) -> bool {
        match key {
            Key::Esc => {
                self.overlay = None;
                true
            }
            Key::Up => {
                if let Some(Overlay::ModelPick { sel, .. }) = &mut self.overlay {
                    *sel = sel.saturating_sub(1);
                }
                true
            }
            Key::Down => {
                let count = match &self.overlay {
                    Some(Overlay::ModelPick { models: super::Loadable::Ready(m), .. }) => m.len(),
                    _ => 0,
                };
                if let Some(Overlay::ModelPick { sel, .. }) = &mut self.overlay {
                    if count > 0 {
                        *sel = (*sel + 1).min(count - 1);
                    }
                }
                true
            }
            Key::Enter => {
                let pick = match &self.overlay {
                    Some(Overlay::ModelPick { models: super::Loadable::Ready(m), sel }) => {
                        m.get(*sel).map(|x| x.id.clone())
                    }
                    _ => None,
                };
                if let Some(id) = pick {
                    self.select_model(id);
                }
                true
            }
            _ => false,
        }
    }

    fn new_branch_key(&mut self, key: Key, mods: Mods) -> bool {
        let nbranches = self
            .rv
            .as_ref()
            .and_then(|rv| rv.branches.ready().map(|b| b.len()))
            .unwrap_or(0);
        let Some(Overlay::NewBranch { name, base }) = &mut self.overlay else { return false };
        match key {
            Key::Esc => {
                self.overlay = None;
                true
            }
            // Up / Down pick the base branch (Left/Right stay with the name's
            // text cursor).
            Key::Up => {
                if nbranches > 0 {
                    *base = (*base + nbranches - 1) % nbranches;
                }
                true
            }
            Key::Down => {
                if nbranches > 0 {
                    *base = (*base + 1) % nbranches;
                }
                true
            }
            Key::Enter => {
                let nm = name.text.trim().to_string();
                let b = *base;
                if nm.is_empty() {
                    self.toast = Some(("enter a branch name".into(), false));
                    return true;
                }
                self.overlay = None;
                self.create_branch(nm, b);
                true
            }
            k => name.handle_key(&k, mods),
        }
    }

    fn newfile_overlay_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(Overlay::NewFile(input)) = &mut self.overlay else { return false };
        match key {
            Key::Esc => {
                self.overlay = None;
                true
            }
            Key::Enter => {
                let path = input.text.trim().to_string();
                self.overlay = None;
                if !path.is_empty() {
                    self.create_new_file(path);
                }
                true
            }
            k => input.handle_key(&k, mods),
        }
    }

    fn open_repo_overlay_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(Overlay::OpenRepo(input)) = &mut self.overlay else { return false };
        match key {
            Key::Esc => {
                self.overlay = None;
                true
            }
            Key::Enter => {
                let name = input.text.trim().trim_matches('/').to_string();
                if name.is_empty() {
                    return true;
                }
                self.overlay = None;
                if name.contains('/') {
                    self.toast = Some((format!("opening {}…", name), false));
                    self.opening_repo = Some(name.clone());
                    let token = self.token.clone();
                    crate::spawn_msg(async move {
                        let result = github::get_repo(&token, &name).await;
                        Msg::RepoOpened { name, result, then_open: None }
                    });
                } else {
                    // Bare name: browse that organization (or user).
                    self.open_org(name);
                }
                true
            }
            k => input.handle_key(&k, mods),
        }
    }

    fn branch_pick_key(&mut self, key: Key) -> bool {
        // `n` swaps the picker for the new-branch modal.
        if key == Key::Char('n') {
            self.open_new_branch_modal();
            return true;
        }
        let count = self
            .rv
            .as_ref()
            .and_then(|rv| rv.branches.ready().map(|b| b.len()))
            .unwrap_or(0);
        let view_h = self.layout.overlay_h.max(1);
        let Some(Overlay::BranchPick { sel, scroll }) = &mut self.overlay else { return false };
        match key {
            Key::Esc => {
                self.overlay = None;
                true
            }
            Key::Up => {
                *sel = sel.saturating_sub(1);
                if *sel < *scroll {
                    *scroll = *sel;
                }
                true
            }
            Key::Down => {
                if count > 0 {
                    *sel = (*sel + 1).min(count - 1);
                }
                if *sel >= *scroll + view_h {
                    *scroll = *sel + 1 - view_h;
                }
                true
            }
            Key::Enter => {
                let pick = self.rv.as_ref().and_then(|rv| {
                    rv.branches.ready().and_then(|b| b.get(*sel)).map(|b| b.name.clone())
                });
                self.overlay = None;
                if let Some(name) = pick {
                    let modified = self
                        .rv
                        .as_ref()
                        .and_then(|rv| rv.file.as_ref())
                        .map(|f| f.editor.modified)
                        .unwrap_or(false);
                    let same = self
                        .rv
                        .as_ref()
                        .map(|rv| rv.branch == name)
                        .unwrap_or(true);
                    if same {
                        return true;
                    }
                    if modified {
                        self.overlay = Some(Overlay::Confirm {
                            msg: format!("discard unsaved edits and switch to {}?", name),
                            action: ConfirmAction::SwitchBranch(name),
                        });
                    } else {
                        self.switch_branch(name);
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn confirm_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(Overlay::Confirm { action, .. }) = &self.overlay else { return false };
        let action = action.clone();
        match key {
            Key::Enter | Key::Char('y') if plain(mods) => {
                self.overlay = None;
                match action {
                    ConfirmAction::LeaveRepo => {
                        self.route = Route::Repos;
                        self.rv = None;
                    }
                    ConfirmAction::SwitchBranch(name) => self.switch_branch(name),
                    ConfirmAction::OpenFile(path) => self.open_file(path),
                    ConfirmAction::OpenRepo { repo, then_open } => {
                        self.open_repo_then(repo, then_open)
                    }
                    ConfirmAction::ApprovePr(number) => self.do_approve(number),
                    ConfirmAction::MergePr { number, method } => self.do_merge(number, method),
                    ConfirmAction::DeleteRun { repo, run_id } => self.do_delete_run(repo, run_id),
                    ConfirmAction::DeleteSecret { repo, name } => self.do_delete_secret(repo, name),
                    ConfirmAction::DeleteVariable { repo, name } => self.do_delete_variable(repo, name),
                    ConfirmAction::DeleteDeployKey { repo, id } => self.do_delete_deploy_key(repo, id),
                }
                true
            }
            Key::Esc | Key::Char('n') if plain(mods) => {
                self.overlay = None;
                true
            }
            _ => false,
        }
    }
}
