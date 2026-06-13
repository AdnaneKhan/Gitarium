//! The open-repo view: its state, opening/switching, and the async results
//! that populate it (branches, tree).

use std::collections::{BTreeMap, HashSet};

use crate::github::{self, Branch, Issue, Job, Pull, Repo, Run, TreeEntry};

use super::issue_detail::Detail;
use super::{App, Loadable, Msg, RepoFocus, Route, Staged, Tab, TreeRow};

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
    /// Drilled-into job logs: (job id, fetched text). Some replaces the jobs
    /// list with a scrollable log view; Esc / back returns to the list.
    pub job_logs: Option<(u64, Loadable<String>)>,
    /// Active in-log search (None when the search box is closed).
    pub log_search: Option<super::LogSearch>,
    pub issues: Loadable<Vec<Issue>>,
    pub issues_sel: usize,
    pub issues_scroll: usize,
    pub pulls: Loadable<Vec<Pull>>,
    pub pulls_sel: usize,
    pub pulls_scroll: usize,
    /// The open issue/PR detail (body + comments + PR merge state). None
    /// while showing a list; Esc closes it back to the list.
    pub detail: Option<Detail>,
    /// File path to open once branches arrive (a global code-search hit
    /// opened this repo). Lives with the RepoView so a superseding open
    /// can't apply it to the wrong repo; consumed in `on_branches`.
    pub pending_open_path: Option<String>,
    /// The staged workspace: path → pending change (edit/add or delete),
    /// committed together via the Git DB API. Cleared on a successful commit
    /// or branch switch. Ordered so the commit + UI list are deterministic.
    pub staged: BTreeMap<String, Staged>,
    /// A staged commit (Git DB blobs→tree→commit→ref) is in flight.
    pub committing: bool,
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
            job_logs: None,
            log_search: None,
            issues: Loadable::Idle,
            issues_sel: 0,
            issues_scroll: 0,
            pulls: Loadable::Idle,
            pulls_sel: 0,
            pulls_scroll: 0,
            detail: None,
            pending_open_path: None,
            staged: BTreeMap::new(),
            committing: false,
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

    /// Open `repo` and, once its branches arrive, jump to `then_open` (a
    /// global code-search hit). The pending path rides the fresh RepoView.
    pub(super) fn open_repo_then(&mut self, repo: Repo, then_open: Option<String>) {
        self.open_repo(repo);
        if then_open.is_some() {
            if let Some(rv) = &mut self.rv {
                rv.pending_open_path = then_open;
            }
        }
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
