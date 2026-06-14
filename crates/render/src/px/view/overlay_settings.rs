//! Renderer for the SettingsForm overlay (create/edit a secret, variable, or
//! deploy key): a vertical stack of labeled inputs (masked for secret values)
//! plus an optional cycling chip. Mirrors `ov_commit`.

use super::*;

impl View {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn ov_settings_form(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, pw: f32, lift: f32, _title: &str) {
        let Some(Overlay::SettingsForm(form)) = &app.overlay else {
            return;
        };
        let focus = form.focus;
        let n = form.n_controls();
        // Snapshot the inputs so `self` is free to draw (the live form is mutated on key/click).
        let inputs: Vec<(String, LineInput, bool)> = form
            .fields
            .iter()
            .map(|f| (f.label.clone(), f.input.clone_shallow(), f.readonly))
            .collect();
        let chip = form.chip.as_ref().map(|c| (c.label.clone(), c.options.clone(), c.sel));
        let submit = form.submit.clone();

        let ph = self.f(132.0) + inputs.len() as f32 * self.f(62.0) + if chip.is_some() { self.f(58.0) } else { 0.0 };
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
        self.overlay_panel(dl, atlas, r, form.title.as_str());

        let lx = r.x + self.f(24.0);
        let fw = r.w - self.f(48.0);
        let mut y = r.y + self.f(60.0);
        for (i, (label, input, readonly)) in inputs.iter().enumerate() {
            self.field_label(dl, atlas, lx, y, label);
            y += self.f(16.0);
            let fr = RectF::new(lx, y, fw, self.f(36.0));
            self.input_field(dl, atlas, input, fr, focus == i);
            if *readonly {
                let tw = dl.text_width(atlas, UI, self.f(10.0), "read-only", 0.0);
                dl.text(atlas, UI, self.f(10.0), fr.right() - tw - self.f(8.0), fr.y + self.f(4.0), "read-only", DIM, 0.0);
            }
            y += self.f(46.0);
        }
        if let Some((label, options, sel)) = &chip {
            self.field_label(dl, atlas, lx, y, &format!("{}  ←/→ or click", label));
            y += self.f(16.0);
            let val = options.get(*sel).cloned().unwrap_or_default();
            let tw = dl.text_width(atlas, UI_BOLD, self.f(12.0), &val, self.f(1.0));
            let cr = RectF::new(lx, y, tw + self.f(24.0), self.f(28.0));
            let hv = self.hover_amt(wid(Z_MENU, 950), cr.contains(self.mouse.0, self.mouse.1));
            let foc = focus == n - 1;
            dl.rrect(cr, self.f(4.0), with_a(CYAN, 0.10 + 0.10 * hv), 1.0);
            dl.border(cr, self.f(4.0), if foc { 2.0 } else { 1.0 }, with_a(CYAN, if foc { 0.9 } else { 0.45 }));
            dl.text(atlas, UI_BOLD, self.f(12.0), cr.x + self.f(12.0), cr.y + self.f(19.0), &val, CYAN, self.f(1.0));
            self.clicks.push((cr, Click::SettingsCycleChip));
        }
        dl.text(atlas, UI, self.f(11.0), lx, r.bottom() - self.f(22.0), &format!("[ENTER] {} · [TAB] next · [ESC] abort", submit), FAINT, self.f(1.5));
    }
}
