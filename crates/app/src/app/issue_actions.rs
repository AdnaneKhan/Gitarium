//! The two outward-facing PR actions the detail view exposes: approve and
//! merge. Both confirm first (they post to GitHub), then spawn the request;
//! the result refreshes the PR's merge state.

use crate::github;

use super::{App, ConfirmAction, Msg, Overlay};

impl App {
    /// Approving requires a token (and that the viewer isn't the author —
    /// GitHub enforces that, surfaced as an error if violated).
    pub(super) fn approve_pr(&mut self) {
        if self.token.is_none() {
            self.toast = Some(("approving requires an access token".into(), true));
            return;
        }
        let Some(d) = self.rv.as_ref().and_then(|rv| rv.detail.as_ref()) else { return };
        if !d.is_pr {
            return;
        }
        let number = d.number;
        self.overlay = Some(Overlay::Confirm {
            msg: format!("approve PR #{}?", number),
            action: ConfirmAction::ApprovePr(number),
        });
    }

    /// Merging requires push access.
    pub(super) fn merge_pr(&mut self) {
        if !self.can_edit_repo() {
            let msg = if self.login.is_none() {
                "sign in to merge PRs"
            } else {
                "view-only: no write access to this repo"
            };
            self.toast = Some((msg.into(), true));
            return;
        }
        let Some(d) = self.rv.as_ref().and_then(|rv| rv.detail.as_ref()) else { return };
        if !d.is_pr {
            return;
        }
        let number = d.number;
        let method = d.merge_method;
        self.overlay = Some(Overlay::Confirm {
            msg: format!("{} PR #{}?", method.label().to_lowercase(), number),
            action: ConfirmAction::MergePr { number, method: method.api().to_string() },
        });
    }

    pub(super) fn do_approve(&mut self, number: u64) {
        let Some(rv) = self.rv.as_ref() else { return };
        if rv.detail.as_ref().map(|d| d.number) != Some(number) {
            return;
        }
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        if let Some(d) = self.rv.as_mut().and_then(|rv| rv.detail.as_mut()) {
            d.action_busy = true;
        }
        self.toast = Some(("submitting approval…".into(), false));
        crate::spawn_msg(async move {
            Msg::PrActed {
                repo: full.clone(),
                number,
                approve: true,
                result: github::approve_pull(&token, &full, number).await.map(|_| "approved".to_string()),
            }
        });
    }

    pub(super) fn do_merge(&mut self, number: u64, method: String) {
        let Some(rv) = self.rv.as_ref() else { return };
        if rv.detail.as_ref().map(|d| d.number) != Some(number) {
            return;
        }
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        if let Some(d) = self.rv.as_mut().and_then(|rv| rv.detail.as_mut()) {
            d.action_busy = true;
        }
        self.toast = Some(("merging…".into(), false));
        crate::spawn_msg(async move {
            Msg::PrActed {
                repo: full.clone(),
                number,
                approve: false,
                result: github::merge_pull(&token, &full, number, &method).await,
            }
        });
    }
}
