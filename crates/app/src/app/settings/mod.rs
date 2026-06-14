//! The Settings tab: section state, lazy-load, and the async results. The
//! two-pane render + nav live in the render crate; the create/edit form types
//! and submit are in `form.rs`; section CRUD in `secrets_vars.rs` and
//! `deploy_keys.rs`; keymap in `keymap.rs`; the form's overlay keys in `form_key.rs`.

use crate::github::{self, DeployKey, SecretMeta, SettingsData, Variable};

use super::{App, Loadable, Msg};
// Re-exported for the sibling submodules below (they `use super::{…}`).
pub(super) use super::{keys::plain, ConfirmAction, Overlay, Tab};

mod deploy_keys;
mod form;
mod form_key;
mod keymap;
mod secrets_vars;

pub use form::{ChipSel, SettingsField, SettingsForm};

/// One section of the settings sidebar. Only the sections implemented so far
/// are present; collaborators/webhooks/environments/branches/rulesets/actions
/// perms arrive in later phases.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    General,
    Secrets,
    Variables,
    DeployKeys,
}

impl SettingsSection {
    pub fn label(self) -> &'static str {
        match self {
            SettingsSection::General => "General",
            SettingsSection::Secrets => "Secrets",
            SettingsSection::Variables => "Variables",
            SettingsSection::DeployKeys => "Deploy keys",
        }
    }
    /// Admin-only sections are hidden from write-only viewers.
    pub fn needs_admin(self) -> bool {
        matches!(self, SettingsSection::DeployKeys)
    }
}

/// The sections shown in the nav, given the viewer's access.
pub fn visible_sections(admin: bool) -> &'static [SettingsSection] {
    match admin {
        true => &[
            SettingsSection::General,
            SettingsSection::Secrets,
            SettingsSection::Variables,
            SettingsSection::DeployKeys,
        ],
        false => &[SettingsSection::General, SettingsSection::Secrets, SettingsSection::Variables],
    }
}

/// Per-section loaded data + nav/list cursors. Lives on `RepoView`.
pub struct SettingsView {
    pub section: SettingsSection,
    pub nav_focused: bool,
    pub nav_sel: usize,
    pub nav_scroll: usize,
    pub list_sel: usize,
    pub list_scroll: usize,
    pub secrets: Loadable<Vec<SecretMeta>>,
    pub variables: Loadable<Vec<Variable>>,
    pub deploy_keys: Loadable<Vec<DeployKey>>,
}

impl Default for SettingsView {
    fn default() -> Self {
        SettingsView {
            section: SettingsSection::General,
            nav_focused: false,
            nav_sel: 0,
            nav_scroll: 0,
            list_sel: 0,
            list_scroll: 0,
            secrets: Loadable::Idle,
            variables: Loadable::Idle,
            deploy_keys: Loadable::Idle,
        }
    }
}

impl App {
    /// Select a nav section, lazy-loading its data on first entry (like the
    /// top-level `switch_tab`).
    pub(crate) fn switch_settings_section(&mut self, section: SettingsSection) {
        // Keep nav_sel in sync with section (click or keyboard both land here).
        let nav_idx = visible_sections(self.is_admin())
            .iter()
            .position(|&s| s == section)
            .unwrap_or(0);
        let load = {
            let Some(rv) = self.rv.as_mut() else { return };
            let s = &mut rv.settings;
            s.section = section;
            s.nav_sel = nav_idx;
            s.list_sel = 0;
            s.list_scroll = 0;
            match section {
                SettingsSection::Secrets => matches!(s.secrets, Loadable::Idle),
                SettingsSection::Variables => matches!(s.variables, Loadable::Idle),
                SettingsSection::DeployKeys => matches!(s.deploy_keys, Loadable::Idle),
                SettingsSection::General => false,
            }
        };
        if load {
            self.load_settings_section(section);
        }
    }

    pub(crate) fn load_settings_section(&mut self, section: SettingsSection) {
        let Some(rv) = &mut self.rv else { return };
        match section {
            SettingsSection::Secrets => rv.settings.secrets = Loadable::Loading,
            SettingsSection::Variables => rv.settings.variables = Loadable::Loading,
            SettingsSection::DeployKeys => rv.settings.deploy_keys = Loadable::Loading,
            SettingsSection::General => return,
        };
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        crate::spawn_msg(async move {
            let result = match section {
                SettingsSection::Secrets => github::list_secrets(&token, &full).await.map(SettingsData::Secrets),
                SettingsSection::Variables => github::list_variables(&token, &full).await.map(SettingsData::Variables),
                SettingsSection::DeployKeys => github::list_deploy_keys(&token, &full).await.map(SettingsData::DeployKeys),
                SettingsSection::General => unreachable!("General is handled before the spawn"),
            };
            Msg::SettingsLoaded { repo: full, section, result }
        });
    }

    pub(crate) fn on_settings_loaded(
        &mut self,
        repo: String,
        section: SettingsSection,
        result: Result<SettingsData, String>,
    ) {
        let Some(rv) = &mut self.rv else { return };
        if rv.repo.full_name != repo {
            return;
        }
        let s = &mut rv.settings;
        match (section, result) {
            (SettingsSection::Secrets, Ok(SettingsData::Secrets(v))) => s.secrets = Loadable::Ready(v),
            (SettingsSection::Secrets, Err(e)) => s.secrets = Loadable::Failed(e),
            (SettingsSection::Variables, Ok(SettingsData::Variables(v))) => s.variables = Loadable::Ready(v),
            (SettingsSection::Variables, Err(e)) => s.variables = Loadable::Failed(e),
            (SettingsSection::DeployKeys, Ok(SettingsData::DeployKeys(v))) => s.deploy_keys = Loadable::Ready(v),
            (SettingsSection::DeployKeys, Err(e)) => s.deploy_keys = Loadable::Failed(e),
            _ => {}
        }
        s.list_sel = 0;
        s.list_scroll = 0;
    }

    /// A mutation finished: toast and refetch the section so the list reflects
    /// the change (simple + correct; optimistic updates can come later).
    pub(crate) fn on_settings_mutated(
        &mut self,
        repo: String,
        section: SettingsSection,
        result: Result<(), String>,
    ) {
        if self.rv.as_ref().map(|rv| rv.repo.full_name != repo).unwrap_or(true) {
            return;
        }
        match result {
            Ok(()) => {
                self.toast = Some(("saved ✓".into(), false));
                self.load_settings_section(section);
            }
            Err(e) => self.toast = Some((e, true)),
        }
    }
}
