//! Repository collaborators: add (username + role chip) and remove. Admin only.

use crate::github;
use crate::ui::lineinput::LineInput;

use super::form::{ChipSel, SettingsAction, SettingsField, SettingsForm, COLLAB_PERMS, COLLAB_ROLES};
use super::{App, ConfirmAction, Msg, Overlay, SettingsSection};

impl App {
    /// `create` → empty username, default role Write. Edit → pre-fills the
    /// selected collaborator's username (read-only) and current role.
    pub(crate) fn open_collaborator_form(&mut self, create: bool) {
        if !self.is_admin() {
            self.toast = Some(("admin access required".into(), true));
            return;
        }
        let Some(rv) = self.rv.as_ref() else { return };
        let repo = rv.repo.full_name.clone();
        let (user_text, readonly, role_sel) = if create {
            (String::new(), false, 2usize)
        } else {
            let c = rv.settings.collaborators.ready().and_then(|v| v.get(rv.settings.list_sel));
            match c {
                None => (String::new(), false, 2),
                Some(c) => {
                    // Map the role string back to a chip index.
                    let i = COLLAB_PERMS.iter().position(|p| *p == c.role()).unwrap_or(2);
                    (c.login.clone(), true, i)
                }
            }
        };
        let mut user = SettingsField { label: "Username".into(), input: LineInput::new(false), readonly };
        if !user_text.is_empty() {
            user.input.insert(&user_text);
        }
        let title = if create { "Add collaborator".into() } else { format!("Edit · {}", user_text) };
        self.overlay = Some(Overlay::SettingsForm(SettingsForm::Simple {
            title,
            submit: "Save".into(),
            section: SettingsSection::Collaborators,
            fields: vec![user],
            chip: Some(ChipSel { label: "Role".into(), options: COLLAB_ROLES.iter().map(|s| s.to_string()).collect(), sel: role_sel }),
            focus: if create { 0 } else { 1 },
            action: SettingsAction::AddCollaborator { repo },
        }));
    }

    pub(crate) fn request_delete_collaborator(&mut self) {
        let Some(rv) = self.rv.as_ref() else { return };
        let Some(c) = rv.settings.collaborators.ready().and_then(|v| v.get(rv.settings.list_sel)) else {
            return;
        };
        let repo = rv.repo.full_name.clone();
        let user = c.login.clone();
        // A pending invite is cancelled via the invitations endpoint (the user
        // isn't a collaborator yet); an accepted collaborator is removed by login.
        self.overlay = Some(match c.invite_id {
            Some(invite_id) => Overlay::Confirm {
                msg: format!("cancel invitation to {}?", user),
                action: ConfirmAction::CancelInvitation { repo, invite_id },
            },
            None => Overlay::Confirm {
                msg: format!("remove {}?", user),
                action: ConfirmAction::RemoveCollaborator { repo, user },
            },
        });
    }

    pub(crate) fn do_remove_collaborator(&mut self, repo: String, user: String) {
        let Some(rv) = self.rv.as_ref() else { return };
        if rv.repo.full_name != repo {
            return;
        }
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        self.toast = Some(("removing…".into(), false));
        crate::spawn_msg(async move {
            let result = github::remove_collaborator(&token, &full, &user).await;
            Msg::SettingsMutated { repo: full, section: SettingsSection::Collaborators, result }
        });
    }

    pub(crate) fn do_cancel_invitation(&mut self, repo: String, invite_id: i64) {
        let Some(rv) = self.rv.as_ref() else { return };
        if rv.repo.full_name != repo {
            return;
        }
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        self.toast = Some(("canceling invite…".into(), false));
        crate::spawn_msg(async move {
            let result = github::cancel_invitation(&token, &full, invite_id).await;
            Msg::SettingsMutated { repo: full, section: SettingsSection::Collaborators, result }
        });
    }
}
