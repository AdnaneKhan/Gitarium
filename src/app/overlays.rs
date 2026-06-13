//! Overlay key dispatch plus the simple overlays (commit message, open
//! repo, branch picker, confirm). The two search palettes live in search.rs.

use crate::github;
use crate::ui::input::{Key, Mods};

use super::keys::plain;
use super::{App, ConfirmAction, Msg, Overlay, Route};

impl App {
    pub(super) fn overlay_key(&mut self, key: Key, mods: Mods) -> bool {
        match &self.overlay {
            None => false,
            Some(Overlay::Help) => {
                self.overlay = None;
                true
            }
            Some(Overlay::Commit(_)) => self.commit_overlay_key(key, mods),
            Some(Overlay::OpenRepo(_)) => self.open_repo_overlay_key(key, mods),
            Some(Overlay::BranchPick { .. }) => self.branch_pick_key(key),
            Some(Overlay::Confirm { .. }) => self.confirm_key(key, mods),
            Some(Overlay::FileSearch { .. }) => self.file_search_key(key, mods),
            Some(Overlay::CodeSearch { .. }) => self.code_search_key(key, mods),
        }
    }

    fn commit_overlay_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(Overlay::Commit(input)) = &mut self.overlay else { return false };
        match key {
            Key::Esc => {
                self.overlay = None;
                true
            }
            Key::Enter => {
                let msg = input.text.trim().to_string();
                if msg.is_empty() {
                    return true;
                }
                self.overlay = None;
                self.commit_file(msg);
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
