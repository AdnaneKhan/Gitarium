//! Renderer for the SettingsForm overlay. `Simple` (secrets/variables/deploy
//! keys/collaborators/general) is a stack of labeled inputs + an optional
//! cycling chip. `Multi` (webhooks) is a URL input + content-type chip + an
//! event toggle list. Mirrors `ov_commit`.

use super::*;

impl View {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn ov_settings_form(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, pw: f32, lift: f32, _title: &str) {
        let multi = matches!(app.overlay, Some(Overlay::SettingsForm(SettingsForm::Multi { .. })));
        if !matches!(app.overlay, Some(Overlay::SettingsForm(_))) {
            return;
        }
        if multi {
            self.draw_settings_multi(app, dl, atlas, w, h, pw, lift);
        } else {
            self.draw_settings_simple(app, dl, atlas, w, h, pw, lift);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_settings_simple(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, pw: f32, lift: f32) {
        let Some(Overlay::SettingsForm(SettingsForm::Simple { title, submit, fields, chip, focus, .. })) = &app.overlay
        else {
            return;
        };
        let title = title.clone();
        let submit = submit.clone();
        let focus = *focus;
        let n = fields.len() + chip.is_some() as usize;
        let inputs: Vec<(String, LineInput, bool)> = fields
            .iter()
            .map(|f| (f.label.clone(), f.input.clone_shallow(), f.readonly))
            .collect();
        let chip = chip.as_ref().map(|c| (c.label.clone(), c.options.clone(), c.sel));

        let ph = self.f(132.0) + inputs.len() as f32 * self.f(62.0) + if chip.is_some() { self.f(58.0) } else { 0.0 };
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
        self.overlay_panel(dl, atlas, r, &title);

        let lx = r.x + self.f(24.0);
        let fw = r.w - self.f(48.0);
        let mut y = r.y + self.f(60.0);
        for (i, (label, input, readonly)) in inputs.iter().enumerate() {
            self.field_label(dl, atlas, lx, y, label);
            y += self.f(16.0);
            let fr = RectF::new(lx, y, fw, self.f(36.0));
            self.input_field(dl, atlas, input, fr, focus == i);
            self.clicks.push((fr, Click::SettingsFocusField(i)));
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

    #[allow(clippy::too_many_arguments)]
    fn draw_settings_multi(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, pw: f32, lift: f32) {
        let Some(Overlay::SettingsForm(SettingsForm::Multi { title, submit, url, content_type, events, focus, .. })) = &app.overlay
        else {
            return;
        };
        let title = title.clone();
        let submit = submit.clone();
        let url = url.clone_shallow();
        let content_type = *content_type;
        let events = events.clone();
        let focus = *focus;

        let row_h = self.f(24.0);
        let ph = self.f(132.0) + self.f(62.0) + self.f(46.0) + events.len() as f32 * row_h;
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
        self.overlay_panel(dl, atlas, r, &title);

        let lx = r.x + self.f(24.0);
        let fw = r.w - self.f(48.0);
        let mut y = r.y + self.f(60.0);
        // URL field (focus 0).
        self.field_label(dl, atlas, lx, y, "Payload URL");
        y += self.f(16.0);
        let url_rect = RectF::new(lx, y, fw, self.f(36.0));
        self.input_field(dl, atlas, &url, url_rect, focus == 0);
        y += self.f(50.0);
        // Content-type chip (focus 1).
        let ct = if content_type == 0 { "json" } else { "form" };
        let ctw = dl.text_width(atlas, UI_BOLD, self.f(12.0), ct, self.f(1.0));
        let cr = RectF::new(lx, y, ctw + self.f(24.0), self.f(28.0));
        let hv = self.hover_amt(wid(Z_MENU, 951), cr.contains(self.mouse.0, self.mouse.1));
        let foc = focus == 1;
        dl.rrect(cr, self.f(4.0), with_a(CYAN, 0.10 + 0.10 * hv), 1.0);
        dl.border(cr, self.f(4.0), if foc { 2.0 } else { 1.0 }, with_a(CYAN, if foc { 0.9 } else { 0.45 }));
        dl.text(atlas, UI_BOLD, self.f(12.0), cr.x + self.f(12.0), cr.y + self.f(19.0), ct, CYAN, self.f(1.0));
        dl.text(atlas, UI, self.f(10.0), cr.right() + self.f(10.0), cr.y + self.f(10.0), "content-type  ←/→ or click", DIM, 0.0);
        self.clicks.push((cr, Click::SettingsCycleContentType));
        y += self.f(46.0);
        // Events toggle list (focus 2..).
        self.field_label(dl, atlas, lx, y, "Events  (Space toggles)");
        y += self.f(18.0);
        dl.push_clip(RectF::new(lx, y, fw, r.bottom() - y - self.f(30.0)));
        for (i, (name, on)) in events.iter().enumerate() {
            let ey = y + i as f32 * row_h;
            let rr = RectF::new(lx, ey - self.f(2.0), fw, row_h - 2.0);
            let mark = if *on { "◉" } else { "○" };
            let col = if *on { CYAN } else { DIM };
            let hv = self.hover_amt(wid(Z_MENU, 960 + i), rr.contains(self.mouse.0, self.mouse.1));
            let a = if focus == 2 + i { 0.10 } else { 0.05 * hv };
            if a > 0.005 {
                dl.rrect(rr, self.f(3.0), with_a(CYAN, a), 1.0);
            }
            dl.text(atlas, MONO, self.f(12.0), lx, ey + self.f(16.0), mark, col, 0.0);
            dl.text(atlas, UI, self.f(12.0), lx + self.f(20.0), ey + self.f(16.0), name, if *on { TEXT } else { with_a(TEXT, 0.6) }, 0.0);
            self.clicks.push((rr, Click::SettingsToggleEvent(i)));
        }
        dl.pop_clip();
        dl.text(atlas, UI, self.f(11.0), lx, r.bottom() - self.f(22.0), &format!("[ENTER] {} · [TAB] next · [SPACE] toggle · [ESC] abort", submit), FAINT, self.f(1.5));
    }
}
