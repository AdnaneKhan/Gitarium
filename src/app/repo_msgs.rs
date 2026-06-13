//! Async results that populate the open-repo view: the repo fetch (from
//! the OpenRepo overlay or a global code-search hit), its branches, and its
//! tree. Each guards against stale responses for a repo since navigated away.

use crate::github::{self, Branch, Repo};

use super::{rebuild_rows, App, ConfirmAction, Loadable, Overlay};

impl App {
    pub(super) fn on_repo_opened(
        &mut self,
        name: String,
        result: Result<Repo, String>,
        then_open: Option<String>,
    ) {
        // Only the most recent async open may act; anything else is a stale
        // response the user has navigated away from.
        if self.opening_repo.as_deref() != Some(name.as_str()) {
            return;
        }
        self.opening_repo = None;
        match result {
            Ok(repo) => {
                let modified = self
                    .rv
                    .as_ref()
                    .and_then(|rv| rv.file.as_ref())
                    .map(|f| f.editor.modified)
                    .unwrap_or(false);
                if modified {
                    self.overlay = Some(Overlay::Confirm {
                        msg: format!("discard unsaved edits and open {}?", repo.full_name),
                        action: ConfirmAction::OpenRepo { repo, then_open },
                    });
                } else {
                    self.open_repo_then(repo, then_open);
                }
            }
            Err(e) => self.toast = Some((e, true)),
        }
    }

    pub(super) fn on_branches(&mut self, repo: String, result: Result<Vec<Branch>, String>) {
        let current = self.rv.as_ref().map(|rv| rv.repo.full_name.clone());
        if current.as_deref() != Some(repo.as_str()) {
            return;
        }
        match result {
            Ok(branches) => {
                if let Some(rv) = &mut self.rv {
                    if !branches.iter().any(|b| b.name == rv.branch) {
                        if let Some(first) = branches.first() {
                            rv.branch = first.name.clone();
                        }
                    }
                    rv.branches = Loadable::Ready(branches);
                }
                self.load_tree();
                // A global code-search hit opened this repo to reach a file;
                // now that the branch is known, load it. Take it first so the
                // mutable borrow ends before open_file re-borrows rv.
                if let Some(path) = self.rv.as_mut().and_then(|rv| rv.pending_open_path.take()) {
                    self.open_file(path);
                }
            }
            Err(e) => {
                if let Some(rv) = &mut self.rv {
                    rv.branches = Loadable::Failed(e.clone());
                    rv.tree = Loadable::Failed(e);
                }
            }
        }
    }

    pub(super) fn on_tree(&mut self, repo: String, result: Result<github::TreeResp, String>) {
        let Some(rv) = &mut self.rv else { return };
        if rv.repo.full_name != repo {
            return;
        }
        match result {
            Ok(mut t) => {
                rv.truncated = t.truncated;
                t.tree.retain(|e| e.kind == "blob" || e.kind == "tree");
                rv.tree = Loadable::Ready(t.tree);
                rv.tree_sel = 0;
                rv.tree_scroll = 0;
                rebuild_rows(rv);
            }
            Err(e) => rv.tree = Loadable::Failed(e),
        }
    }
}
