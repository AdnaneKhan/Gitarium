//! Keys for the open SettingsForm overlay. `Simple` (secrets/variables/deploy
//! keys/collaborators/general): Tab/↑/↓ move focus across fields + chip, ←/→
//! cycle the chip. `Multi` (webhooks): same focus model but focus 0 = URL,
//! 1 = content-type, 2.. = event index, and Space toggles an event.

use crate::ui::input::{Key, Mods};

use super::form::{HOOK_CT, SettingsForm};
use super::{App, Overlay};

impl App {
    pub(crate) fn settings_form_key(&mut self, key: Key, mods: Mods) -> bool {
        let multi = matches!(self.overlay, Some(Overlay::SettingsForm(SettingsForm::Multi { .. })));
        if multi {
            self.settings_multi_key(key, mods)
        } else {
            self.settings_simple_key(key, mods)
        }
    }

    fn settings_simple_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(Overlay::SettingsForm(SettingsForm::Simple { fields, chip, focus, .. })) = &mut self.overlay
        else {
            return false;
        };
        let n = (fields.len() + chip.is_some() as usize).max(1);
        let on_chip = chip.is_some() && *focus == n - 1;
        match key {
            Key::Esc => {
                self.overlay = None;
                true
            }
            Key::Enter => {
                self.submit_settings_form();
                true
            }
            Key::Tab | Key::Down => {
                *focus = (*focus + 1) % n;
                true
            }
            Key::BackTab | Key::Up => {
                *focus = (*focus + n - 1) % n;
                true
            }
            Key::Right | Key::Char(' ') if on_chip => {
                if let Some(c) = chip.as_mut() {
                    let m = c.options.len();
                    if m > 0 {
                        c.sel = (c.sel + 1) % m;
                    }
                }
                true
            }
            Key::Left if on_chip => {
                if let Some(c) = chip.as_mut() {
                    let m = c.options.len();
                    if m > 0 {
                        c.sel = (c.sel + m - 1) % m;
                    }
                }
                true
            }
            _ if on_chip => true,
            k => {
                let f = *focus;
                if f < fields.len() && !fields[f].readonly {
                    fields[f].input.handle_key(&k, mods)
                } else {
                    false
                }
            }
        }
    }

    fn settings_multi_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(Overlay::SettingsForm(SettingsForm::Multi { url, content_type, events, focus, .. })) =
            &mut self.overlay
        else {
            return false;
        };
        let n = (2 + events.len()).max(1);
        match key {
            Key::Esc => {
                self.overlay = None;
                true
            }
            Key::Enter => {
                self.submit_settings_form();
                true
            }
            Key::Tab | Key::Down => {
                *focus = (*focus + 1) % n;
                true
            }
            Key::BackTab | Key::Up => {
                *focus = (*focus + n - 1) % n;
                true
            }
            Key::Right if *focus == 1 => {
                let m = HOOK_CT.len().max(1);
                *content_type = (*content_type + 1) % m;
                true
            }
            Key::Left if *focus == 1 => {
                let m = HOOK_CT.len().max(1);
                *content_type = (*content_type + m - 1) % m;
                true
            }
            Key::Char(' ') if *focus >= 2 => {
                let i = *focus - 2;
                if i < events.len() {
                    events[i].1 = !events[i].1;
                }
                true
            }
            k => {
                if *focus == 0 {
                    url.handle_key(&k, mods)
                } else {
                    true // swallow on the content-type chip / event rows
                }
            }
        }
    }

    /// Click a simple-form input field → move focus to it (mirrors Tab
    /// landing on that field). Out-of-range indices are ignored.
    pub(crate) fn settings_focus_field(&mut self, i: usize) {
        if let Some(Overlay::SettingsForm(SettingsForm::Simple { fields, focus, .. })) = &mut self.overlay {
            if i < fields.len() {
                *focus = i;
            }
        }
    }

    /// Click the simple form's chip → cycle its selection + focus it (mirrors
    /// the ←/→ keys; focusing it makes those keys act on it next).
    pub(crate) fn settings_cycle_chip(&mut self) {
        if let Some(Overlay::SettingsForm(SettingsForm::Simple { fields, chip, focus, .. })) = &mut self.overlay {
            if let Some(c) = chip {
                let m = c.options.len();
                if m > 0 {
                    c.sel = (c.sel + 1) % m;
                }
                *focus = fields.len(); // the chip is the last control
            }
        }
    }

    /// Click on the webhook form's content-type chip → cycle + focus it
    /// (mirrors the ←/→ keys).
    pub(crate) fn settings_cycle_content_type(&mut self) {
        if let Some(Overlay::SettingsForm(SettingsForm::Multi { content_type, focus, .. })) = &mut self.overlay {
            let m = HOOK_CT.len().max(1);
            *content_type = (*content_type + 1) % m;
            *focus = 1;
        }
    }

    /// Click a webhook event row → toggle + focus it (mirrors Space).
    pub(crate) fn settings_toggle_event(&mut self, i: usize) {
        if let Some(Overlay::SettingsForm(SettingsForm::Multi { events, focus, .. })) = &mut self.overlay {
            if let Some(ev) = events.get_mut(i) {
                ev.1 = !ev.1;
                *focus = 2 + i;
            }
        }
    }
}
