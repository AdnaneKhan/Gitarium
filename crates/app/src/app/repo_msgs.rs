//! Async results that populate the open-repo view: the repo fetch (from
//! the OpenRepo overlay or a global code-search hit), its branches, and its
//! tree. Each guards against stale responses for a repo since navigated away.

use crate::github::{self, Branch, CommitRef, Repo, BRANCH_PER_PAGE};

use super::{rebuild_rows, App, ConfirmAction, Loadable, Overlay};

impl App {
    /// A new branch was created from the modal: register it at the base head
    /// and switch the view to it.
    pub(super) fn on_branch_created(
        &mut self,
        repo: String,
        name: String,
        sha: String,
        result: Result<(), String>,
    ) {
        match &result {
            Ok(()) => self.toast = Some((format!("branch {} created ✓", name), false)),
            Err(e) => self.toast = Some((format!("create branch failed: {}", e), true)),
        }
        if result.is_err() {
            return;
        }
        let same = self.rv.as_ref().map(|rv| rv.repo.full_name == repo).unwrap_or(false);
        if !same {
            return;
        }
        if let Some(rv) = &mut self.rv {
            if let Loadable::Ready(branches) = &mut rv.branches {
                if !branches.iter().any(|b| b.name == name) {
                    branches.push(Branch { name: name.clone(), commit: CommitRef { sha } });
                }
            }
        }
        self.switch_branch(name);
    }

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

    /// The explicit default-branch fetch landed. Merge it in (so it's always
    /// selectable and its sha is known) and kick off the initial tree load.
    pub(super) fn on_default_branch(&mut self, repo: String, result: Result<Branch, String>) {
        if self.rv.as_ref().map(|rv| rv.repo.full_name.as_str()) != Some(repo.as_str()) {
            return;
        }
        match result {
            Ok(branch) => {
                if let Some(rv) = &mut self.rv {
                    match &mut rv.branches {
                        Loadable::Ready(bs) if !bs.iter().any(|b| b.name == branch.name) => bs.insert(0, branch.clone()),
                        Loadable::Ready(_) => {}
                        _ => rv.branches = Loadable::Ready(vec![branch.clone()]),
                    }
                    rv.branch = branch.name;
                }
                self.kick_initial_tree();
            }
            Err(_) => {
                // Couldn't fetch the default directly (private/transient/etc.):
                // fall back to whatever the paged list gives us.
                if let Some(rv) = &mut self.rv {
                    rv.default_failed = true;
                }
                self.ensure_tree_from_branches();
            }
        }
    }

    pub(super) fn on_branches(&mut self, repo: String, page: usize, result: Result<Vec<Branch>, String>) {
        if self.rv.as_ref().map(|rv| rv.repo.full_name.as_str()) != Some(repo.as_str()) {
            return;
        }
        match result {
            Ok(batch) => {
                if let Some(rv) = &mut self.rv {
                    rv.branches_loading = false;
                    rv.branch_page = page.max(rv.branch_page);
                    rv.branches_more = batch.len() == BRANCH_PER_PAGE;
                    match &mut rv.branches {
                        // Append, skipping any names already present (the
                        // default branch may have been prepended already).
                        Loadable::Ready(existing) => {
                            let have: std::collections::HashSet<&str> = existing.iter().map(|b| b.name.as_str()).collect();
                            let fresh: Vec<Branch> = batch.into_iter().filter(|b| !have.contains(b.name.as_str())).collect();
                            existing.extend(fresh);
                        }
                        _ => rv.branches = Loadable::Ready(batch),
                    }
                }
                // Only page 1 is allowed to start the tree, and only as a
                // fallback — the default-branch fetch is the authority for
                // which branch (and sha) the tree should load.
                if page <= 1 && self.rv.as_ref().map(|rv| rv.default_failed).unwrap_or(false) {
                    self.ensure_tree_from_branches();
                }
            }
            Err(e) => {
                if let Some(rv) = &mut self.rv {
                    rv.branches_loading = false;
                    // Keep any branches already in hand (e.g. the default);
                    // only surface a hard failure when we have nothing.
                    if !matches!(rv.branches, Loadable::Ready(_)) {
                        rv.branches = Loadable::Failed(e.clone());
                        rv.tree = Loadable::Failed(e);
                    }
                }
            }
        }
    }

    /// Resolve the active branch from the loaded list and start the tree —
    /// the fallback used when the explicit default-branch fetch fails.
    fn ensure_tree_from_branches(&mut self) {
        let Some(rv) = &mut self.rv else { return };
        if rv.tree_started {
            return;
        }
        let Some(branches) = rv.branches.ready() else { return };
        let Some(first) = branches.first() else { return };
        if !branches.iter().any(|b| b.name == rv.branch) {
            rv.branch = first.name.clone();
        }
        self.kick_initial_tree();
    }

    /// Start the initial tree load exactly once, then consume any pending
    /// global-search path. Assumes `rv.branch` resolves to a known sha.
    fn kick_initial_tree(&mut self) {
        match &mut self.rv {
            Some(rv) if !rv.tree_started => rv.tree_started = true,
            _ => return,
        }
        self.load_tree();
        // A global code-search hit opened this repo to reach a file; load it
        // now that the branch is known. Take it first so the mutable borrow
        // ends before open_file re-borrows rv.
        if let Some(path) = self.rv.as_mut().and_then(|rv| rv.pending_open_path.take()) {
            self.open_file(path);
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
