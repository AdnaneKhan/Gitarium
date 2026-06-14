//! Keys for the open SettingsForm overlay: Tab/↑/↓ move focus across the
//! fields and the chip; ←/→ (or click) cycle the chip; Enter submits; Esc
//! cancels. Mirrors commit_overlay_key. Enter doesn't touch `form`, so the
//! borrow releases and submit_settings_form can consume the overlay.

use crate::ui::input::{Key, Mods};

use super::{App, Overlay};

impl App {
    pub(crate) fn settings_form_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(Overlay::SettingsForm(form)) = &mut self.overlay else {
            return false;
        };
        let n = form.n_controls().max(1);
        let on_chip = form.chip.is_some() && form.focus == n - 1;
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
                form.focus = (form.focus + 1) % n;
                true
            }
            Key::BackTab | Key::Up => {
                form.focus = (form.focus + n - 1) % n;
                true
            }
            Key::Right | Key::Char(' ') if on_chip => {
                if let Some(c) = &mut form.chip {
                    let m = c.options.len();
                    if m > 0 {
                        c.sel = (c.sel + 1) % m;
                    }
                }
                true
            }
            Key::Left if on_chip => {
                if let Some(c) = &mut form.chip {
                    let m = c.options.len();
                    if m > 0 {
                        c.sel = (c.sel + m - 1) % m;
                    }
                }
                true
            }
            _ if on_chip => true, // swallow keys while the chip is focused
            k => {
                let f = form.focus;
                if f < form.fields.len() && !form.fields[f].readonly {
                    form.fields[f].input.handle_key(&k, mods)
                } else {
                    false
                }
            }
        }
    }
}
