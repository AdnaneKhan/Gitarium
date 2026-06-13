//! The issue/PR detail view: state, opening from a list row, the async fetches
//! (comments; for PRs merge state / reviews / checks), and key handling.
//! Approve/merge live in `issue_actions.rs`, search in `issue_search.rs`.

use crate::github::{self, CheckRun, Comment, Label, Pull, Review};
use crate::ui::input::{Key, Mods};

use super::keys::plain;
use super::{App, Loadable, LogSearch, Msg, RepoFocus};

/// How a merge lands; cycled by the detail view's method chip.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}

impl MergeMethod {
    pub fn next(self) -> Self {
        match self {
            MergeMethod::Merge => MergeMethod::Squash,
            MergeMethod::Squash => MergeMethod::Rebase,
            MergeMethod::Rebase => MergeMethod::Merge,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            MergeMethod::Merge => "MERGE COMMIT",
            MergeMethod::Squash => "SQUASH",
            MergeMethod::Rebase => "REBASE",
        }
    }
    pub fn api(self) -> &'static str {
        match self {
            MergeMethod::Merge => "merge",
            MergeMethod::Squash => "squash",
            MergeMethod::Rebase => "rebase",
        }
    }
}

/// The open issue or PR. Title/body/author/state are seeded from the list row
/// (no extra request); comments and the PR-only fields are fetched.
pub struct Detail {
    pub number: u64,
    pub is_pr: bool,
    pub title: String,
    pub body: String,
    pub author: String,
    pub state: String, // open | closed | merged
    pub created_at: String,
    pub labels: Vec<Label>,
    pub comments: Loadable<Vec<Comment>>,
    /// Row scroll offset into the wrapped body+comments content (left column).
    pub scroll: usize,
    /// Row scroll offset into the PR meta column (checks / reviews); PR-only.
    pub meta_scroll: usize,
    // PR-only (Idle for issues):
    pub pr: Loadable<Pull>,
    pub reviews: Loadable<Vec<Review>>,
    pub checks: Loadable<Vec<CheckRun>>,
    pub merge_method: MergeMethod,
    /// An approve/merge request is in flight.
    pub action_busy: bool,
    /// In-page search; `matches` (rendered-row indices) is filled by the view.
    pub search: Option<LogSearch>,
}

impl App {
    pub(super) fn open_issue_detail(&mut self, idx: usize) {
        let Some(rv) = self.rv.as_ref() else { return };
        let Some(issue) = rv.issues.ready().and_then(|v| v.get(idx)) else { return };
        let number = issue.number;
        let detail = Detail {
            number,
            is_pr: false,
            title: issue.title.clone(),
            body: issue.body.clone().unwrap_or_default(),
            author: issue.user.as_ref().map(|u| u.login.clone()).unwrap_or_default(),
            state: issue.state.clone(),
            created_at: issue.created_at.clone(),
            labels: issue.labels.clone(),
            comments: Loadable::Loading,
            scroll: 0,
            meta_scroll: 0,
            pr: Loadable::Idle,
            reviews: Loadable::Idle,
            checks: Loadable::Idle,
            merge_method: MergeMethod::Merge,
            action_busy: false,
            search: None,
        };
        self.rv.as_mut().unwrap().detail = Some(detail);
        self.load_comments(number);
    }

    pub(super) fn open_pull_detail(&mut self, idx: usize) {
        let Some(rv) = self.rv.as_ref() else { return };
        let Some(pull) = rv.pulls.ready().and_then(|v| v.get(idx)) else { return };
        let number = pull.number;
        let head = pull.head.as_ref().map(|h| h.sha.clone()).unwrap_or_default();
        let state = if pull.merged { "merged".into() } else { pull.state.clone() };
        let detail = Detail {
            number,
            is_pr: true,
            title: pull.title.clone(),
            body: pull.body.clone().unwrap_or_default(),
            author: pull.user.as_ref().map(|u| u.login.clone()).unwrap_or_default(),
            state,
            created_at: pull.created_at.clone(),
            labels: pull.labels.clone(),
            comments: Loadable::Loading,
            scroll: 0,
            meta_scroll: 0,
            pr: Loadable::Loading,
            reviews: Loadable::Loading,
            checks: Loadable::Loading,
            merge_method: MergeMethod::Merge,
            action_busy: false,
            search: None,
        };
        self.rv.as_mut().unwrap().detail = Some(detail);
        self.load_comments(number);
        self.load_pr_meta(number, head);
    }

    pub(super) fn close_detail(&mut self) {
        if let Some(rv) = &mut self.rv {
            rv.detail = None;
        }
    }

    fn load_comments(&mut self, number: u64) {
        let Some(rv) = self.rv.as_ref() else { return };
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        crate::spawn_msg(async move {
            Msg::Comments {
                repo: full.clone(),
                number,
                result: github::list_comments(&token, &full, number).await,
            }
        });
    }

    /// Fetch the PR's computed merge state, reviews, and head-commit checks.
    pub(super) fn load_pr_meta(&mut self, number: u64, head_sha: String) {
        let Some(rv) = self.rv.as_ref() else { return };
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        let (f1, f2) = (full.clone(), full.clone());
        crate::spawn_msg(async move {
            Msg::PullLoaded { repo: f1.clone(), number, result: github::get_pull(&token, &f1, number).await }
        });
        let token2 = self.token.clone();
        crate::spawn_msg(async move {
            Msg::Reviews { repo: f2.clone(), number, result: github::list_reviews(&token2, &f2, number).await }
        });
        if head_sha.is_empty() {
            return;
        }
        let token3 = self.token.clone();
        crate::spawn_msg(async move {
            Msg::Checks {
                repo: full.clone(),
                number,
                result: github::list_check_runs(&token3, &full, &head_sha).await,
            }
        });
    }

    pub(super) fn detail_key(&mut self, key: Key, mods: Mods) -> bool {
        // The in-page search box owns keys while it's open.
        if self.rv.as_ref().and_then(|rv| rv.detail.as_ref()).is_some_and(|d| d.search.is_some()) {
            return self.detail_search_key(key, mods);
        }
        let h = self.layout.detail_h.max(1);
        let Some(rv) = self.rv.as_mut() else { return false };
        let Some(d) = rv.detail.as_mut() else { return false };
        match key {
            Key::Esc => {
                rv.detail = None;
                rv.focus = RepoFocus::Tree;
            }
            Key::Char('/') if plain(mods) => self.open_detail_search(),
            Key::Char('?') if plain(mods) => self.overlay = Some(super::Overlay::Help),
            Key::Char('i') if plain(mods) => self.open_agent(),
            Key::Char('a') if plain(mods) && d.is_pr => self.approve_pr(),
            Key::Char('m') if plain(mods) && d.is_pr => self.merge_pr(),
            Key::Up => d.scroll = d.scroll.saturating_sub(1),
            Key::Down => d.scroll += 1,
            Key::PageUp => d.scroll = d.scroll.saturating_sub(h),
            Key::PageDown => d.scroll += h,
            Key::Home => d.scroll = 0,
            _ => return false,
        }
        true
    }
}
