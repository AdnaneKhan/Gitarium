//! Async results that populate the open issue/PR detail: comments, the PR's
//! merge state, reviews, checks, and the outcome of an approve/merge. Each
//! guards on the live detail's number (and repo) so a stale response for a
//! detail since closed or replaced is dropped.

use crate::github::{CheckRun, Comment, Pull, Review};

use super::{App, Loadable};

impl App {
    /// Run `f` against the live detail iff it matches `repo` + `number`.
    fn with_detail<F: FnOnce(&mut super::issue_detail::Detail)>(
        &mut self,
        repo: &str,
        number: u64,
        f: F,
    ) {
        let Some(rv) = &mut self.rv else { return };
        if rv.repo.full_name != repo {
            return;
        }
        if let Some(d) = rv.detail.as_mut() {
            if d.number == number {
                f(d);
            }
        }
    }

    pub(super) fn on_comments(
        &mut self,
        repo: String,
        number: u64,
        result: Result<Vec<Comment>, String>,
    ) {
        self.with_detail(&repo, number, |d| {
            d.comments = match result {
                Ok(v) => Loadable::Ready(v),
                Err(e) => Loadable::Failed(e),
            };
        });
    }

    pub(super) fn on_pull_loaded(&mut self, repo: String, number: u64, result: Result<Pull, String>) {
        self.with_detail(&repo, number, |d| match result {
            Ok(p) => {
                if p.merged {
                    d.state = "merged".into();
                } else if !p.state.is_empty() {
                    d.state = p.state.clone();
                }
                d.pr = Loadable::Ready(p);
            }
            Err(e) => d.pr = Loadable::Failed(e),
        });
    }

    pub(super) fn on_reviews(
        &mut self,
        repo: String,
        number: u64,
        result: Result<Vec<Review>, String>,
    ) {
        self.with_detail(&repo, number, |d| {
            d.reviews = match result {
                Ok(v) => Loadable::Ready(v),
                Err(e) => Loadable::Failed(e),
            };
        });
    }

    pub(super) fn on_checks(
        &mut self,
        repo: String,
        number: u64,
        result: Result<Vec<CheckRun>, String>,
    ) {
        self.with_detail(&repo, number, |d| {
            d.checks = match result {
                Ok(v) => Loadable::Ready(v),
                Err(e) => Loadable::Failed(e),
            };
        });
    }

    pub(super) fn on_pr_acted(
        &mut self,
        repo: String,
        number: u64,
        approve: bool,
        result: Result<String, String>,
    ) {
        let ok = result.is_ok();
        match &result {
            Ok(msg) => self.toast = Some((format!("{} ✓", msg), false)),
            Err(e) => self.toast = Some((e.clone(), true)),
        }
        let head = self
            .rv
            .as_ref()
            .and_then(|rv| rv.detail.as_ref())
            .and_then(|d| d.pr.ready())
            .and_then(|p| p.head.as_ref().map(|h| h.sha.clone()))
            .unwrap_or_default();
        self.with_detail(&repo, number, |d| d.action_busy = false);
        if !ok {
            return;
        }
        // Refresh the PR's merge state/reviews/checks after the action.
        let still_open = self
            .rv
            .as_ref()
            .and_then(|rv| rv.detail.as_ref())
            .map(|d| d.number == number)
            .unwrap_or(false);
        if still_open {
            if let Some(d) = self.rv.as_mut().and_then(|rv| rv.detail.as_mut()) {
                d.pr = Loadable::Loading;
                d.reviews = Loadable::Loading;
            }
            self.load_pr_meta(number, head);
        }
        // A merged PR drops off the open list — refresh it.
        if !approve {
            self.load_pulls();
        }
    }
}
