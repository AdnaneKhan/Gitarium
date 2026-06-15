//! Keymap for the Settings tab: global keys, nav-list movement (when the nav
//! pane is focused), content-list movement, and the per-section `n`/`e`/`d`/`r`
//! actions. The section create/edit/delete methods live in `secrets_vars.rs`
//! and `deploy_keys.rs` as `impl App` blocks.

use crate::ui::input::{Key, Mods};

use super::{plain, visible_sections, App, Overlay, SettingsSection, Tab};

impl App {
    pub(crate) fn settings_key(&mut self, key: Key, mods: Mods) -> bool {
        let (nav_focused, section, count) = self.settings_snapshot();
        match key {
            Key::Char('?') if plain(mods) => {
                self.overlay = Some(Overlay::Help);
                return true;
            }
            Key::Char(',') if plain(mods) => return true, // already on Settings
            Key::Tab => {
                if let Some(rv) = &mut self.rv {
                    rv.settings.nav_focused = !rv.settings.nav_focused;
                }
                return true;
            }
            _ => {}
        }
        if nav_focused {
            return self.settings_nav_step(key);
        }
        match key {
            Key::Esc => {
                self.switch_tab(Tab::Code);
                true
            }
            Key::Up => {
                self.settings_list_move(-1, count);
                true
            }
            Key::Down => {
                self.settings_list_move(1, count);
                true
            }
            Key::PageUp => {
                self.settings_list_move(-8, count);
                true
            }
            Key::PageDown => {
                self.settings_list_move(8, count);
                true
            }
            Key::Home => {
                self.settings_list_set(0);
                true
            }
            Key::End => {
                self.settings_list_set(count.saturating_sub(1));
                true
            }
            Key::Char('r') if plain(mods) => {
                self.load_settings_section(section);
                true
            }
            Key::Char('n') if plain(mods) => {
                self.open_section_create(section);
                true
            }
            Key::Char('e') if plain(mods) => {
                self.open_section_edit(section);
                true
            }
            Key::Enter => {
                self.open_section_edit(section);
                true
            }
            Key::Char('d') if plain(mods) => {
                self.request_section_delete(section);
                true
            }
            _ => false,
        }
    }

    fn settings_snapshot(&self) -> (bool, SettingsSection, usize) {
        let Some(rv) = self.rv.as_ref() else {
            return (false, SettingsSection::General, 0);
        };
        let s = &rv.settings;
        let count = match s.section {
            SettingsSection::Secrets => s.secrets.ready().map(|v| v.len()).unwrap_or(0),
            SettingsSection::Variables => s.variables.ready().map(|v| v.len()).unwrap_or(0),
            SettingsSection::DeployKeys => s.deploy_keys.ready().map(|v| v.len()).unwrap_or(0),
            SettingsSection::Collaborators => s.collaborators.ready().map(|v| v.len()).unwrap_or(0),
            SettingsSection::Webhooks => s.webhooks.ready().map(|v| v.len()).unwrap_or(0),
            SettingsSection::General => 0,
        };
        (s.nav_focused, s.section, count)
    }

    /// Move the nav selection; switching sections lazy-loads the target.
    fn settings_nav_step(&mut self, key: Key) -> bool {
        let secs = visible_sections(self.is_admin());
        let n = secs.len();
        let idx = self.rv.as_ref().map(|rv| rv.settings.nav_sel).unwrap_or(0);
        let new = match key {
            Key::Up => idx.saturating_sub(1),
            Key::Down if n > 0 => (idx + 1).min(n - 1),
            Key::PageUp => idx.saturating_sub(8),
            Key::PageDown if n > 0 => (idx + 8).min(n - 1),
            Key::Home => 0,
            Key::End => n.saturating_sub(1),
            _ => return false,
        };
        if let Some(&sec) = secs.get(new) {
            self.switch_settings_section(sec);
        }
        true
    }

    fn settings_list_move(&mut self, delta: i32, count: usize) {
        if count == 0 {
            return;
        }
        if let Some(rv) = &mut self.rv {
            let max = count - 1;
            let cur = rv.settings.list_sel as i32;
            rv.settings.list_sel = (cur + delta).clamp(0, max as i32) as usize;
        }
    }

    fn settings_list_set(&mut self, idx: usize) {
        if let Some(rv) = &mut self.rv {
            rv.settings.list_sel = idx;
        }
    }

    pub(crate) fn open_section_create(&mut self, section: SettingsSection) {
        match section {
            SettingsSection::Secrets => self.open_secret_form(true),
            SettingsSection::Variables => self.open_variable_form(true),
            SettingsSection::DeployKeys => self.open_deploy_key_form(),
            SettingsSection::Collaborators => self.open_collaborator_form(true),
            SettingsSection::Webhooks => self.open_webhook_form(true),
            SettingsSection::General => self.open_general_form(),
        }
    }

    pub(crate) fn open_section_edit(&mut self, section: SettingsSection) {
        match section {
            SettingsSection::Secrets => self.open_secret_form(false),
            SettingsSection::Variables => self.open_variable_form(false),
            SettingsSection::DeployKeys => {
                self.toast = Some(("deploy keys can't be edited — delete and re-add".into(), true));
            }
            SettingsSection::Collaborators => self.open_collaborator_form(false),
            SettingsSection::Webhooks => self.open_webhook_form(false),
            SettingsSection::General => self.open_general_form(),
        }
    }

    pub(crate) fn request_section_delete(&mut self, section: SettingsSection) {
        match section {
            SettingsSection::Secrets => self.request_delete_secret(),
            SettingsSection::Variables => self.request_delete_variable(),
            SettingsSection::DeployKeys => self.request_delete_deploy_key(),
            SettingsSection::Collaborators => self.request_delete_collaborator(),
            SettingsSection::Webhooks => self.request_delete_webhook(),
            SettingsSection::General => self.request_delete_repo(),
        }
    }
}
