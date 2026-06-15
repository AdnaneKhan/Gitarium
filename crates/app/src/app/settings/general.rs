//! General settings: edit metadata (name / description / default-branch) and
//! the danger zone (archive / delete). Edits and archive return the fresh Repo
//! via `Msg::RepoMetaUpdated` so `rv.repo` updates in place; delete leaves the
//! repo screen on success.

use crate::github;
use crate::ui::lineinput::LineInput;

use super::form::{ChipSel, SettingsAction, SettingsField, SettingsForm};
use super::{App, ConfirmAction, Msg, Overlay, Route, SettingsSection};

impl App {
    pub(crate) fn open_general_form(&mut self) {
        let Some(rv) = self.rv.as_ref() else { return };
        if !rv.repo.permissions.as_ref().map(|p| p.admin).unwrap_or(false) {
            self.toast = Some(("admin access required to edit repo settings".into(), true));
            return;
        }
        let repo = rv.repo.full_name.clone();
        let mk = |label: &str, val: &str| {
            let mut f = SettingsField { label: label.into(), input: LineInput::new(false), readonly: false };
            if !val.is_empty() {
                f.input.insert(val);
            }
            f
        };
        let name = mk("Name", &rv.repo.name);
        let desc = mk("Description", rv.repo.description.as_deref().unwrap_or(""));
        // Default-branch chip cycles among the loaded branches (no chip if none).
        let chip = rv.branches.ready().map(|bs| {
            let options: Vec<String> = bs.iter().map(|b| b.name.clone()).collect();
            let sel = options.iter().position(|b| b == &rv.repo.default_branch).unwrap_or(0);
            ChipSel { label: "Default branch".into(), options, sel }
        });
        self.overlay = Some(Overlay::SettingsForm(SettingsForm::Simple {
            title: "Edit repository".into(),
            submit: "Save".into(),
            section: SettingsSection::General,
            fields: vec![name, desc],
            chip,
            focus: 0,
            action: SettingsAction::EditGeneral { repo },
        }));
    }

    pub(crate) fn request_archive_repo(&mut self) {
        let Some(rv) = self.rv.as_ref() else { return };
        let repo = rv.repo.full_name.clone();
        let verb = if rv.repo.archived { "un-archive" } else { "archive" };
        self.overlay = Some(Overlay::Confirm {
            msg: format!("{} {}?", verb, repo),
            action: ConfirmAction::ArchiveRepo { repo },
        });
    }

    /// `archive_repo` only archives; un-archiving isn't wired (rare).
    pub(crate) fn do_archive_repo(&mut self, repo: String) {
        let token = self.token.clone();
        let full = repo.clone();
        self.toast = Some(("archiving…".into(), false));
        crate::spawn_msg(async move {
            let result = github::archive_repo(&token, &full).await;
            Msg::RepoMetaUpdated { repo: full, result }
        });
    }

    pub(crate) fn request_delete_repo(&mut self) {
        let Some(rv) = self.rv.as_ref() else { return };
        let repo = rv.repo.full_name.clone();
        self.overlay = Some(Overlay::Confirm {
            msg: format!("PERMANENTLY DELETE {}? This cannot be undone.", repo),
            action: ConfirmAction::DeleteRepo { repo },
        });
    }

    pub(crate) fn do_delete_repo(&mut self, repo: String) {
        let token = self.token.clone();
        let full = repo.clone();
        self.toast = Some(("deleting repository…".into(), false));
        crate::spawn_msg(async move {
            let result = github::delete_repo(&token, &full).await;
            Msg::RepoDeleted { repo: full, result }
        });
    }

    /// A metadata edit or archive finished → swap in the fresh Repo.
    pub(crate) fn on_repo_meta_updated(&mut self, repo: String, result: Result<github::Repo, String>) {
        let Some(rv) = &mut self.rv else { return };
        if rv.repo.full_name != repo {
            return;
        }
        match result {
            Ok(r) => {
                rv.repo = r;
                self.toast = Some(("saved ✓".into(), false));
            }
            Err(e) => self.toast = Some((e, true)),
        }
    }

    pub(crate) fn on_repo_deleted(&mut self, repo: String, result: Result<(), String>) {
        match result {
            Ok(()) => {
                self.toast = Some(("repository deleted".into(), false));
                self.route = Route::Repos;
                self.rv = None;
            }
            Err(e) => {
                if self.rv.as_ref().map(|rv| rv.repo.full_name == repo).unwrap_or(false) {
                    self.toast = Some((e, true));
                }
            }
        }
    }
}
