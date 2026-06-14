//! Deleting a workflow run from the Actions tab. The call is irreversible, so
//! it confirms first (like merge/approve); on success the run is removed from
//! the in-memory list (no refetch flash) and any jobs/logs loaded for it are
//! dropped.

use crate::github;

use super::{App, ConfirmAction, Loadable, Msg, Overlay};

impl App {
    /// Open the confirm for deleting `run_id`. Re-checks write access even
    /// though the context menu only offers the item when allowed.
    pub(super) fn request_delete_run(&mut self, run_id: u64) {
        if !self.can_edit_repo() {
            let msg = if self.login.is_none() {
                "sign in to delete workflow runs"
            } else {
                "view-only: no write access to this repo"
            };
            self.toast = Some((msg.into(), true));
            return;
        }
        let Some(rv) = self.rv.as_ref() else { return };
        let repo = rv.repo.full_name.clone();
        // A friendlier label than the raw database id.
        let label = rv
            .runs
            .ready()
            .and_then(|rs| rs.iter().find(|r| r.id == run_id))
            .map(|r| {
                let title = r
                    .display_title
                    .clone()
                    .or_else(|| r.name.clone())
                    .unwrap_or_else(|| format!("run {}", r.id));
                format!("#{} {}", r.run_number, title)
            })
            .unwrap_or_else(|| format!("#{}", run_id));
        self.overlay = Some(Overlay::Confirm {
            msg: format!("delete workflow run {}?", label),
            action: ConfirmAction::DeleteRun { repo, run_id },
        });
    }

    /// Fire the DELETE (confirmation accepted). Guarded against a stale
    /// confirm for a repo no longer open.
    pub(super) fn do_delete_run(&mut self, repo: String, run_id: u64) {
        let Some(rv) = self.rv.as_ref() else { return };
        if rv.repo.full_name != repo {
            return;
        }
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        self.toast = Some(("deleting run…".into(), false));
        crate::spawn_msg(async move {
            Msg::RunDeleted {
                repo: full.clone(),
                run_id,
                result: github::delete_workflow_run(&token, &full, run_id).await,
            }
        });
    }

    pub(super) fn on_run_deleted(&mut self, repo: String, run_id: u64, result: Result<(), String>) {
        if self.rv.as_ref().map(|rv| rv.repo.full_name != repo).unwrap_or(true) {
            return;
        }
        match result {
            Ok(()) => {
                self.toast = Some(("run deleted ✓".into(), false));
                let Some(rv) = &mut self.rv else { return };
                // Drop jobs/logs if they belonged to the deleted run.
                if rv.jobs.as_ref().map(|(id, _)| *id == run_id).unwrap_or(false) {
                    rv.jobs = None;
                }
                rv.job_logs = None;
                // Remove the run from the list straight away — it's gone on
                // GitHub, so an immediate local removal beats a refetch (which
                // would flash "FETCHING RUNS…" and reset the cursor).
                let idx = match &rv.runs {
                    Loadable::Ready(runs) => runs.iter().position(|r| r.id == run_id),
                    _ => None,
                };
                if let Some(i) = idx {
                    if let Loadable::Ready(runs) = &mut rv.runs {
                        runs.remove(i);
                    }
                    // Keep the cursor on the same run: if the deleted one sat
                    // above it, shift up one, then clamp to the new tail.
                    if i < rv.runs_sel {
                        rv.runs_sel -= 1;
                    }
                    let max = match &rv.runs {
                        Loadable::Ready(runs) => runs.len().saturating_sub(1),
                        _ => 0,
                    };
                    if rv.runs_sel > max {
                        rv.runs_sel = max;
                    }
                }
            }
            Err(e) => self.toast = Some((e, true)),
        }
    }
}
