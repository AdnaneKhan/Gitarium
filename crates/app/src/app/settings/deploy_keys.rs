//! Deploy keys: the add form (title + key + read-only chip) and delete. Admin
//! only. Deploy keys can't be edited (GitHub treats them immutable) — edit is a
//! delete-and-readd.

use crate::github;
use crate::ui::lineinput::LineInput;

use super::form::{ChipSel, SettingsAction, SettingsField, SettingsForm};
use super::{App, ConfirmAction, Msg, Overlay, SettingsSection};

impl App {
    pub(crate) fn open_deploy_key_form(&mut self) {
        if !self.is_admin() {
            self.toast = Some(("admin access required to manage deploy keys".into(), true));
            return;
        }
        let Some(rv) = self.rv.as_ref() else { return };
        let repo = rv.repo.full_name.clone();
        let title = SettingsField { label: "Title".into(), input: LineInput::new(false), readonly: false };
        let key = SettingsField { label: "Public key (ssh-… / ecdsa-…)".into(), input: LineInput::new(false), readonly: false };
        self.overlay = Some(Overlay::SettingsForm(SettingsForm::Simple {
            title: "New deploy key".into(),
            submit: "Add key".into(),
            section: SettingsSection::DeployKeys,
            fields: vec![title, key],
            chip: Some(ChipSel {
                label: "Access".into(),
                options: vec!["Read/Write".into(), "Read-only".into()],
                sel: 0,
            }),
            focus: 0,
            action: SettingsAction::AddDeployKey { repo },
        }));
    }

    pub(crate) fn request_delete_deploy_key(&mut self) {
        let Some(rv) = self.rv.as_ref() else { return };
        let Some((id, title)) = rv
            .settings
            .deploy_keys
            .ready()
            .and_then(|v| v.get(rv.settings.list_sel))
            .map(|k| (k.id, k.title.clone()))
        else {
            return;
        };
        let repo = rv.repo.full_name.clone();
        let label = title.unwrap_or_else(|| format!("#{}", id));
        self.overlay = Some(Overlay::Confirm {
            msg: format!("delete deploy key {}?", label),
            action: ConfirmAction::DeleteDeployKey { repo, id },
        });
    }

    pub(crate) fn do_delete_deploy_key(&mut self, repo: String, id: i64) {
        let Some(rv) = self.rv.as_ref() else { return };
        if rv.repo.full_name != repo {
            return;
        }
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        self.toast = Some(("deleting key…".into(), false));
        crate::spawn_msg(async move {
            let result = github::delete_deploy_key(&token, &full, id).await;
            Msg::SettingsMutated { repo: full, section: SettingsSection::DeployKeys, result }
        });
    }
}
