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
    /// Highest branch page appended so far (0 before page 1 lands).
    pub branch_page: usize,
    /// Another page of branches may exist (the last page came back full).
    pub branches_more: bool,
    /// A branch-page fetch is in flight (suppresses duplicate lazy loads).
    pub branches_loading: bool,
    /// The initial tree load has been kicked off; guards the default-branch /
    /// page-1 race so the tree loads exactly once, for the right branch.
    pub tree_started: bool,
    /// The explicit default-branch fetch failed, so page 1 must stand in to
    /// pick the active branch and start the tree.
    pub default_failed: bool,
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
            branch_page: 0,
            branches_more: false,
            branches_loading: false,
            tree_started: false,
            default_failed: false,
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
        let default = repo.default_branch.clone();
        self.rv = Some(RepoView::new(repo));
        if let Some(rv) = &mut self.rv {
            rv.branches_loading = true;
        }
        self.route = Route::Repo;
        let token = self.token.clone();
        // The default branch, fetched directly so it's always present and its
        // head sha is known — even on repos with thousands of branches where
        // it sorts past the first page. The first page populates the picker.
        let (f1, f2) = (full.clone(), full.clone());
        let token2 = token.clone();
        crate::spawn_msg(async move {
            Msg::DefaultBranch { repo: f1.clone(), result: github::get_branch(&token2, &f1, &default).await }
        });
        crate::spawn_msg(async move {
            Msg::Branches { repo: f2.clone(), page: 1, result: github::list_branches(&token, &f2, 1).await }
        });
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
}
