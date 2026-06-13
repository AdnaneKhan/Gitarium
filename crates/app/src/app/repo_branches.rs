//! Branch operations on the open repo: lazy pagination of the branch list,
//! the new-branch modal, ref creation, and switching the active branch.

use crate::github;

use super::{App, Msg};

impl App {
    /// Pull the next page of branches into the picker, if one may exist and a
    /// fetch isn't already in flight.
    pub fn load_more_branches(&mut self) {
        let Some(rv) = self.rv.as_ref() else { return };
        if rv.branches_loading || !rv.branches_more {
            return;
        }
        let next = rv.branch_page + 1;
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        if let Some(rv) = &mut self.rv {
            rv.branches_loading = true;
        }
        crate::spawn_msg(async move {
            Msg::Branches { repo: full.clone(), page: next, result: github::list_branches(&token, &full, next).await }
        });
    }

    /// Open the new-branch modal, basing it on the currently-active branch.
    pub(super) fn open_new_branch_modal(&mut self) {
        if !self.can_edit_repo() {
            let msg = if self.login.is_none() {
                "sign in to create a branch"
            } else {
                "view-only: no write access to this repo"
            };
            self.toast = Some((msg.into(), true));
            return;
        }
        let Some(rv) = self.rv.as_ref() else { return };
        let base = rv
            .branches
            .ready()
            .and_then(|bs| bs.iter().position(|b| b.name == rv.branch))
            .unwrap_or(0);
        self.overlay = Some(super::Overlay::NewBranch {
            name: crate::ui::lineinput::LineInput::new(false),
            base,
        });
    }

    /// Create `name` from the base branch at `base_idx` (an empty branch
    /// pointing at that branch's head), then switch to it.
    pub(super) fn create_branch(&mut self, name: String, base_idx: usize) {
        if self.token.is_none() {
            self.toast = Some(("creating a branch requires an access token".into(), true));
            return;
        }
        let Some(rv) = self.rv.as_ref() else { return };
        let Some(branches) = rv.branches.ready() else { return };
        if branches.iter().any(|b| b.name == name) {
            self.toast = Some((format!("{} already exists", name), true));
            return;
        }
        let Some(base) = branches.get(base_idx) else { return };
        let sha = base.commit.sha.clone();
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        crate::spawn_msg(async move {
            let result = github::create_ref(&token, &full, &format!("refs/heads/{}", name), &sha)
                .await
                .map(|_| ());
            Msg::BranchCreated { repo: full, name, sha, result }
        });
        self.toast = Some(("creating branch…".into(), false));
    }

    pub(super) fn switch_branch(&mut self, name: String) {
        let Some(rv) = &mut self.rv else { return };
        rv.branch = name;
        rv.file = None;
        rv.file_loading = None;
        rv.expanded.clear();
        rv.tree_sel = 0;
        rv.tree_scroll = 0;
        // Staged changes are relative to the old branch's tree; drop them.
        rv.staged.clear();
        self.load_tree();
    }
}
