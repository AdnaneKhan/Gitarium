//! The open-repo view: its state, opening/switching, and the async results
//! that populate it (branches, tree).

use std::collections::HashSet;

use crate::github::{self, Branch, Job, Repo, Run, TreeEntry};

use super::{
    rebuild_rows, App, ConfirmAction, Loadable, Msg, Overlay, RepoFocus, Route, Tab, TreeRow,
};

pub struct RepoView {
    pub repo: Repo,
    pub branch: String,
    pub branches: Loadable<Vec<Branch>>,
    pub tree: Loadable<Vec<TreeEntry>>,
    pub rows: Vec<TreeRow>,
    pub expanded: HashSet<String>,
    pub tree_sel: usize,
    pub tree_scroll: usize,
    pub truncated: bool,
    pub file: Option<super::OpenFile>,
    pub file_loading: Option<String>,
    pub tab: Tab,
    pub focus: RepoFocus,
    pub runs: Loadable<Vec<Run>>,
    pub runs_sel: usize,
    pub runs_scroll: usize,
    pub jobs: Option<(u64, Loadable<Vec<Job>>)>,
    pub jobs_scroll: usize,
}

impl RepoView {
    fn new(repo: Repo) -> Self {
        let branch = repo.default_branch.clone();
        RepoView {
            repo,
            branch,
            branches: Loadable::Loading,
            tree: Loadable::Loading,
            rows: Vec::new(),
            expanded: HashSet::new(),
            tree_sel: 0,
            tree_scroll: 0,
            truncated: false,
            file: None,
            file_loading: None,
            tab: Tab::Code,
            focus: RepoFocus::Tree,
            runs: Loadable::Idle,
            runs_sel: 0,
            runs_scroll: 0,
            jobs: None,
            jobs_scroll: 0,
        }
    }

    pub(super) fn branch_sha(&self) -> Option<String> {
        self.branches
            .ready()?
            .iter()
            .find(|b| b.name == self.branch)
            .map(|b| b.commit.sha.clone())
    }
}

impl App {
    pub(super) fn open_repo(&mut self, repo: Repo) {
        // Supersedes any async open still in flight.
        self.opening_repo = None;
        let full = repo.full_name.clone();
        self.rv = Some(RepoView::new(repo));
        self.route = Route::Repo;
        let token = self.token.clone();
        let full2 = full.clone();
        crate::spawn_msg(async move {
            Msg::Branches {
                repo: full2.clone(),
                result: github::list_branches(&token, &full2).await,
            }
        });
        let _ = full;
    }

    pub(super) fn load_tree(&mut self) {
        let Some(rv) = &mut self.rv else { return };
        let Some(sha) = rv.branch_sha() else {
            rv.tree = Loadable::Failed("branch not found".into());
            return;
        };
        rv.tree = Loadable::Loading;
        rv.rows.clear();
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        crate::spawn_msg(async move {
            Msg::Tree {
                repo: full.clone(),
                result: github::get_tree(&token, &full, &sha).await,
            }
        });
    }

    pub(super) fn switch_branch(&mut self, name: String) {
        let Some(rv) = &mut self.rv else { return };
        rv.branch = name;
        rv.file = None;
        rv.file_loading = None;
        rv.expanded.clear();
        rv.tree_sel = 0;
        rv.tree_scroll = 0;
        self.load_tree();
    }

    pub(super) fn on_repo_opened(&mut self, name: String, result: Result<Repo, String>) {
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
                        action: ConfirmAction::OpenRepo(repo),
                    });
                } else {
                    self.open_repo(repo);
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
