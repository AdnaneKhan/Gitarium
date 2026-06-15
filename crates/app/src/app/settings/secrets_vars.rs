//! Actions secrets and variables: the create/edit forms and delete (confirm →
//! spawn). Secret values are masked in the form; the seal-encryption itself
//! runs in submit_settings_form (form.rs).

use crate::github;
use crate::ui::lineinput::LineInput;

use super::form::{SettingsAction, SettingsField, SettingsForm};
use super::{App, ConfirmAction, Msg, Overlay, SettingsSection};

impl App {
    pub(crate) fn open_secret_form(&mut self, create: bool) {
        if !self.can_edit_repo() {
            self.toast = Some(("sign in to manage secrets".into(), true));
            return;
        }
        let Some(rv) = self.rv.as_ref() else { return };
        let repo = rv.repo.full_name.clone();
        let (name, readonly) = if create {
            (String::new(), false)
        } else {
            (
                rv.settings
                    .secrets
                    .ready()
                    .and_then(|v| v.get(rv.settings.list_sel))
                    .map(|s| s.name.clone())
                    .unwrap_or_default(),
                true,
            )
        };
        let mut nm = SettingsField { label: "Name".into(), input: LineInput::new(false), readonly };
        if !name.is_empty() {
            nm.input.insert(&name);
        }
        let val = SettingsField { label: "Value".into(), input: LineInput::new(true), readonly: false };
        self.overlay = Some(Overlay::SettingsForm(SettingsForm::Simple {
            title: if create { "New secret".into() } else { format!("Edit secret · {}", name) },
            submit: "Save secret".into(),
            section: SettingsSection::Secrets,
            fields: vec![nm, val],
            chip: None,
            focus: if create { 0 } else { 1 },
            action: SettingsAction::SaveSecret { repo },
        }));
    }

    pub(crate) fn open_variable_form(&mut self, create: bool) {
        if !self.can_edit_repo() {
            self.toast = Some(("sign in to manage variables".into(), true));
            return;
        }
        let Some(rv) = self.rv.as_ref() else { return };
        let repo = rv.repo.full_name.clone();
        let (name, value, readonly) = if create {
            (String::new(), String::new(), false)
        } else {
            rv.settings
                .variables
                .ready()
                .and_then(|v| v.get(rv.settings.list_sel))
                .map(|x| (x.name.clone(), x.value.clone(), true))
                .unwrap_or_default()
        };
        let mut nm = SettingsField { label: "Name".into(), input: LineInput::new(false), readonly };
        if !name.is_empty() {
            nm.input.insert(&name);
        }
        let mut val = SettingsField { label: "Value".into(), input: LineInput::new(false), readonly: false };
        if !value.is_empty() {
            val.input.insert(&value);
        }
        self.overlay = Some(Overlay::SettingsForm(SettingsForm::Simple {
            title: if create { "New variable".into() } else { format!("Edit variable · {}", name) },
            submit: "Save variable".into(),
            section: SettingsSection::Variables,
            fields: vec![nm, val],
            chip: None,
            focus: 0,
            action: SettingsAction::SaveVariable { repo, create },
        }));
    }

    pub(crate) fn request_delete_secret(&mut self) {
        let Some(rv) = self.rv.as_ref() else { return };
        let Some(name) = rv
            .settings
            .secrets
            .ready()
            .and_then(|v| v.get(rv.settings.list_sel))
            .map(|s| s.name.clone())
        else {
            return;
        };
        let repo = rv.repo.full_name.clone();
        self.overlay = Some(Overlay::Confirm {
            msg: format!("delete secret {}?", name),
            action: ConfirmAction::DeleteSecret { repo, name },
        });
    }

    pub(crate) fn do_delete_secret(&mut self, repo: String, name: String) {
        let Some(rv) = self.rv.as_ref() else { return };
        if rv.repo.full_name != repo {
            return;
        }
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        self.toast = Some(("deleting secret…".into(), false));
        crate::spawn_msg(async move {
            let result = github::delete_secret(&token, &full, &name).await;
            Msg::SettingsMutated { repo: full, section: SettingsSection::Secrets, result }
        });
    }

    pub(crate) fn request_delete_variable(&mut self) {
        let Some(rv) = self.rv.as_ref() else { return };
        let Some(name) = rv
            .settings
            .variables
            .ready()
            .and_then(|v| v.get(rv.settings.list_sel))
            .map(|s| s.name.clone())
        else {
            return;
        };
        let repo = rv.repo.full_name.clone();
        self.overlay = Some(Overlay::Confirm {
            msg: format!("delete variable {}?", name),
            action: ConfirmAction::DeleteVariable { repo, name },
        });
    }

    pub(crate) fn do_delete_variable(&mut self, repo: String, name: String) {
        let Some(rv) = self.rv.as_ref() else { return };
        if rv.repo.full_name != repo {
            return;
        }
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        self.toast = Some(("deleting variable…".into(), false));
        crate::spawn_msg(async move {
            let result = github::delete_variable(&token, &full, &name).await;
            Msg::SettingsMutated { repo: full, section: SettingsSection::Variables, result }
        });
    }
}
