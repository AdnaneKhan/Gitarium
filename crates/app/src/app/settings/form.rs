//! The create/edit form overlay types and the submit dispatcher. One shape —
//! N labeled `LineInput`s (masked for secret values) plus an optional cycling
//! chip — covers secrets, variables, and deploy keys. Phases 2–3 add `Multi`
//! (webhook events) and `Json` (rulesets / branch protection) variants.

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

/// A single-select chip cycled by ←/→ or click (deploy-key read/write).
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
}

pub struct SettingsForm {
    pub title: String,
    pub submit: String,
    pub section: SettingsSection,
    pub fields: Vec<SettingsField>,
    pub chip: Option<ChipSel>,
    /// Focused control: 0..fields.len() is a field; the chip is the last index.
    pub focus: usize,
    pub action: SettingsAction,
}

impl SettingsForm {
    pub fn n_controls(&self) -> usize {
        self.fields.len() + self.chip.is_some() as usize
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
        let repo = match &form.action {
            SettingsAction::SaveSecret { repo }
            | SettingsAction::SaveVariable { repo, .. }
            | SettingsAction::AddDeployKey { repo } => repo.clone(),
        };
        let Some(rv) = self.rv.as_ref() else { return };
        if rv.repo.full_name != repo {
            return;
        }
        let full = rv.repo.full_name.clone();
        let token = self.token.clone();
        let action = form.action.clone();
        let field = |i: usize| form.fields.get(i).map(|f| f.input.text.clone()).unwrap_or_default();
        let chip_sel = form.chip.as_ref().map(|c| c.sel).unwrap_or(0);

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
                // Chip option 0 = Read/Write, 1 = Read-only.
                let read_only = chip_sel == 1;
                self.toast = Some(("adding key…".into(), false));
                crate::spawn_msg(async move {
                    let result = github::add_deploy_key(&token, &full, &title, &key, read_only).await;
                    Msg::SettingsMutated { repo: full, section: SettingsSection::DeployKeys, result }
                });
            }
        }
    }
}
