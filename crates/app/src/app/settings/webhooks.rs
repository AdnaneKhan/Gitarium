//! Repository webhooks: create/edit (Multi form: URL + content-type + events)
//! and delete. Admin only. The event catalog is `HOOK_EVENTS` (form.rs).

use crate::github;
use crate::ui::lineinput::LineInput;

use super::form::{HOOK_EVENTS, SettingsAction, SettingsForm};
use super::{App, ConfirmAction, Msg, Overlay, SettingsSection};

impl App {
    /// `create` → empty URL, content-type json, "push" pre-selected. Edit →
    /// pre-fills the selected webhook's URL, content-type, and current events.
    pub(crate) fn open_webhook_form(&mut self, create: bool) {
        if !self.is_admin() {
            self.toast = Some(("admin access required".into(), true));
            return;
        }
        let Some(rv) = self.rv.as_ref() else { return };
        let repo = rv.repo.full_name.clone();
        let (title, action, url_text, content_type, selected) = if create {
            (
                "New webhook".to_string(),
                SettingsAction::CreateWebhook { repo },
                String::new(),
                0usize,
                vec!["push".to_string()],
            )
        } else {
            let Some(hook) = rv.settings.webhooks.ready().and_then(|v| v.get(rv.settings.list_sel)) else {
                return;
            };
            let ct = if hook.config.content_type.as_deref() == Some("form") { 1 } else { 0 };
            (
                format!("Edit webhook #{}", hook.id),
                SettingsAction::UpdateWebhook { repo, id: hook.id },
                hook.config.url.clone().unwrap_or_default(),
                ct,
                hook.events.clone(),
            )
        };
        let mut url = LineInput::new(false);
        if !url_text.is_empty() {
            url.insert(&url_text);
        }
        let events: Vec<(String, bool)> = HOOK_EVENTS
            .iter()
            .map(|e| ((*e).to_string(), selected.iter().any(|s| s == e)))
            .collect();
        self.overlay = Some(Overlay::SettingsForm(SettingsForm::Multi {
            title,
            submit: "Save".into(),
            section: SettingsSection::Webhooks,
            url,
            content_type,
            events,
            focus: 0,
            action,
        }));
    }

    pub(crate) fn request_delete_webhook(&mut self) {
        let Some(rv) = self.rv.as_ref() else { return };
        let Some(id) = rv.settings.webhooks.ready().and_then(|v| v.get(rv.settings.list_sel)).map(|h| h.id) else {
            return;
        };
        let repo = rv.repo.full_name.clone();
        self.overlay = Some(Overlay::Confirm {
            msg: format!("delete webhook #{}?", id),
            action: ConfirmAction::DeleteWebhook { repo, id },
        });
    }

    pub(crate) fn do_delete_webhook(&mut self, repo: String, id: i64) {
        let Some(rv) = self.rv.as_ref() else { return };
        if rv.repo.full_name != repo {
            return;
        }
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        self.toast = Some(("deleting webhook…".into(), false));
        crate::spawn_msg(async move {
            let result = github::delete_webhook(&token, &full, id).await;
            Msg::SettingsMutated { repo: full, section: SettingsSection::Webhooks, result }
        });
    }
}
