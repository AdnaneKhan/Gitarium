//! The create/edit form overlay types and the submit dispatcher. `Simple`
//! (N labeled inputs + an optional single-select chip) covers secrets,
//! variables, deploy keys, collaborators, and general. `Multi` (a URL field +
//! content-type chip + a multi-select event toggle list) is for webhooks.

use crate::github;
use crate::ui::lineinput::LineInput;

use super::{App, Msg, Overlay};
use super::SettingsSection;

/// One labeled text field. `input.masked` hides the value (secret values); the
/// field is non-editable when `readonly` (e.g. a secret's name on update).
pub struct SettingsField {
    pub label: String,
    pub input: LineInput,
    pub readonly: bool,
}

/// A single-select chip cycled by ←/→ or click (deploy-key read/write,
/// collaborator role, default branch, webhook content-type).
pub struct ChipSel {
    pub label: String,
    pub options: Vec<String>,
    pub sel: usize,
}

/// What a submitted form does. Carries just enough to pick the endpoint.
#[derive(Clone)]
pub enum SettingsAction {
    /// PUT a secret (create-or-update); name from field[0], sealed value from field[1].
    SaveSecret { repo: String },
    /// Create (POST) or update (PATCH) a variable; name from field[0].
    SaveVariable { repo: String, create: bool },
    /// POST a deploy key; title/key from fields, read-only from the chip.
    AddDeployKey { repo: String },
    /// PATCH repo metadata (name/description/default-branch).
    EditGeneral { repo: String },
    /// PUT a collaborator with a permission (the role chip's selected option).
    AddCollaborator { repo: String },
    /// POST a webhook (events from the Multi form's toggles).
    CreateWebhook { repo: String },
    /// PATCH a webhook (existing id).
    UpdateWebhook { repo: String, id: i64 },
}

impl SettingsAction {
    pub fn repo(&self) -> &str {
        match self {
            SettingsAction::SaveSecret { repo }
            | SettingsAction::SaveVariable { repo, .. }
            | SettingsAction::AddDeployKey { repo }
            | SettingsAction::EditGeneral { repo }
            | SettingsAction::AddCollaborator { repo }
            | SettingsAction::CreateWebhook { repo }
            | SettingsAction::UpdateWebhook { repo, .. } => repo,
        }
    }
}

/// Collaborator-role options (display) → GitHub permission strings (API).
pub const COLLAB_ROLES: [&str; 5] = ["Read", "Triage", "Write", "Maintain", "Admin"];
pub const COLLAB_PERMS: [&str; 5] = ["read", "triage", "write", "maintain", "admin"];
/// Webhook content-type options.
pub const HOOK_CT: [&str; 2] = ["json", "form"];
/// A curated webhook event catalog for the Multi form.
pub const HOOK_EVENTS: [&str; 14] = [
    "push",
    "pull_request",
    "issues",
    "issue_comment",
    "pull_request_review",
    "pull_request_review_comment",
    "release",
    "check_run",
    "deployment",
    "status",
    "fork",
    "star",
    "label",
    "milestone",
];

pub enum SettingsForm {
    Simple {
        title: String,
        submit: String,
        section: SettingsSection,
        fields: Vec<SettingsField>,
        chip: Option<ChipSel>,
        /// Focused control: 0..fields.len() is a field; the chip is the last index.
        focus: usize,
        action: SettingsAction,
    },
    /// A webhook form: URL input + content-type chip + event toggle list.
    /// `focus`: 0 = url, 1 = content-type, 2.. = event index (focus-2).
    Multi {
        title: String,
        submit: String,
        section: SettingsSection,
        url: LineInput,
        content_type: usize,
        events: Vec<(String, bool)>,
        focus: usize,
        action: SettingsAction,
    },
}

impl SettingsForm {
    pub fn title(&self) -> &str {
        match self {
            SettingsForm::Simple { title, .. } | SettingsForm::Multi { title, .. } => title,
        }
    }
    /// Total focusable controls (for Tab cycling).
    pub fn n_controls(&self) -> usize {
        match self {
            SettingsForm::Simple { fields, chip, .. } => fields.len() + chip.is_some() as usize,
            SettingsForm::Multi { events, .. } => 2 + events.len(),
        }
    }
}

impl App {
    /// Read the open form, run its action, close it. Secret values are
    /// seal-encrypted against the repo's public key before the PUT.
    pub(crate) fn submit_settings_form(&mut self) {
        let form = match self.overlay.take() {
            Some(Overlay::SettingsForm(f)) => f,
            other => {
                self.overlay = other;
                return;
            }
        };
        match form {
            SettingsForm::Simple { action, fields, chip, .. } => self.submit_simple(action, fields, chip),
            SettingsForm::Multi { action, url, content_type, events, .. } => {
                self.submit_multi(action, url, content_type, events)
            }
        }
    }

    fn submit_simple(&mut self, action: SettingsAction, fields: Vec<SettingsField>, chip: Option<ChipSel>) {
        let Some(rv) = self.rv.as_ref() else { return };
        if rv.repo.full_name != action.repo() {
            return;
        }
        let full = rv.repo.full_name.clone();
        let token = self.token.clone();
        let field = |i: usize| fields.get(i).map(|f| f.input.text.clone()).unwrap_or_default();
        let chip_sel = chip.as_ref().map(|c| c.sel).unwrap_or(0);
        match action {
            SettingsAction::SaveSecret { .. } => {
                let name = field(0).trim().to_string();
                if name.is_empty() {
                    self.toast = Some(("secret name required".into(), true));
                    return;
                }
                let value = field(1);
                self.toast = Some(("saving secret…".into(), false));
                crate::spawn_msg(async move {
                    let result: Result<(), String> = async {
                        let pk = github::get_public_key(&token, &full).await?;
                        let enc = crate::crypto::seal_secret(&pk.key, &value)?;
                        github::put_secret(&token, &full, &name, &enc, &pk.key_id).await?;
                        Ok(())
                    }
                    .await;
                    Msg::SettingsMutated { repo: full, section: SettingsSection::Secrets, result }
                });
            }
            SettingsAction::SaveVariable { create, .. } => {
                let name = field(0).trim().to_string();
                if name.is_empty() {
                    self.toast = Some(("variable name required".into(), true));
                    return;
                }
                let value = field(1);
                self.toast = Some(("saving variable…".into(), false));
                crate::spawn_msg(async move {
                    let result = if create {
                        github::create_variable(&token, &full, &name, &value).await
                    } else {
                        github::update_variable(&token, &full, &name, &value).await
                    };
                    Msg::SettingsMutated { repo: full, section: SettingsSection::Variables, result }
                });
            }
            SettingsAction::AddDeployKey { .. } => {
                let title = field(0).trim().to_string();
                let key = field(1).trim().to_string();
                if key.is_empty() {
                    self.toast = Some(("public key required".into(), true));
                    return;
                }
                let read_only = chip_sel == 1;
                self.toast = Some(("adding key…".into(), false));
                crate::spawn_msg(async move {
                    let result = github::add_deploy_key(&token, &full, &title, &key, read_only).await;
                    Msg::SettingsMutated { repo: full, section: SettingsSection::DeployKeys, result }
                });
            }
            SettingsAction::AddCollaborator { .. } => {
                let user = field(0).trim().to_string();
                if user.is_empty() {
                    self.toast = Some(("username required".into(), true));
                    return;
                }
                let perm = COLLAB_PERMS[chip_sel.min(COLLAB_PERMS.len() - 1)];
                self.toast = Some(("adding collaborator…".into(), false));
                crate::spawn_msg(async move {
                    let result = github::add_collaborator(&token, &full, &user, perm).await;
                    Msg::SettingsMutated { repo: full, section: SettingsSection::Collaborators, result }
                });
            }
            SettingsAction::EditGeneral { .. } => {
                let name = field(0).trim().to_string();
                let description = field(1);
                let default_branch = chip.as_ref().and_then(|c| c.options.get(c.sel).cloned()).unwrap_or_default();
                self.toast = Some(("saving…".into(), false));
                crate::spawn_msg(async move {
                    let result = github::update_repo(&token, &full, &name, &description, "", &default_branch).await;
                    Msg::RepoMetaUpdated { repo: full, result }
                });
            }
            SettingsAction::CreateWebhook { .. } | SettingsAction::UpdateWebhook { .. } => {
                unreachable!("webhooks use the Multi form")
            }
        }
    }

    fn submit_multi(&mut self, action: SettingsAction, url: LineInput, content_type: usize, events: Vec<(String, bool)>) {
        let Some(rv) = self.rv.as_ref() else { return };
        if rv.repo.full_name != action.repo() {
            return;
        }
        let full = rv.repo.full_name.clone();
        let token = self.token.clone();
        let u = url.text.trim().to_string();
        if u.is_empty() {
            self.toast = Some(("webhook URL required".into(), true));
            return;
        }
        let ct = HOOK_CT[content_type.min(HOOK_CT.len() - 1)];
        let selected: Vec<String> = events.iter().filter(|(_, on)| *on).map(|(n, _)| n.clone()).collect();
        match action {
            SettingsAction::CreateWebhook { .. } => {
                self.toast = Some(("creating webhook…".into(), false));
                crate::spawn_msg(async move {
                    let result = github::create_webhook(&token, &full, &u, ct, &selected, true).await;
                    Msg::SettingsMutated { repo: full, section: SettingsSection::Webhooks, result }
                });
            }
            SettingsAction::UpdateWebhook { id, .. } => {
                self.toast = Some(("saving webhook…".into(), false));
                crate::spawn_msg(async move {
                    let result = github::update_webhook(&token, &full, id, &u, ct, &selected, true).await;
                    Msg::SettingsMutated { repo: full, section: SettingsSection::Webhooks, result }
                });
            }
            _ => unreachable!("only webhooks use the Multi form"),
        }
    }
}
