//! The Settings tab: a fixed-width left nav of permission-gated sections and
//! a right content pane that dispatches to the per-section renderers in
//! `settings_content.rs`.

use super::*;

impl View {
    pub(super) fn settings_tab(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, top: f32, bottom: f32) {
        let section = app.rv.as_ref().map(|rv| rv.settings.section).unwrap_or(SettingsSection::General);
        let secs = visible_sections(app.is_admin());
        let nav_w = self.f(220.0);
        let nav = RectF::new(self.f(16.0), top, nav_w, bottom - top);
        let content = RectF::new(nav.right() + self.f(12.0), top, w - nav.right() - self.f(28.0), bottom - top);
        self.panel(dl, nav);
        self.panel(dl, content);

        // Nav header + section list.
        dl.text(atlas, UI, self.f(12.0), nav.x + self.f(14.0), top + self.f(24.0), "SETTINGS", DIM, self.f(2.5));
        let row_h = self.f(30.0);
        for (i, sec) in secs.iter().enumerate() {
            let y = top + self.f(40.0) + i as f32 * row_h;
            let rr = RectF::new(nav.x + self.f(8.0), y, nav.w - self.f(16.0), row_h - 2.0);
            let on = *sec == section;
            let hv = self.hover_amt(wid(Z_SET, i), rr.contains(self.hot.0, self.hot.1));
            let a = if on { 0.14 } else { 0.05 * hv };
            if a > 0.005 {
                dl.rrect(rr, self.f(3.0), with_a(CYAN, a), 1.0);
            }
            let col = if on { CYAN } else { with_a(TEXT, 0.6 + 0.3 * hv) };
            let font = if on { UI_BOLD } else { UI };
            dl.text(atlas, font, self.f(13.0), rr.x + self.f(10.0), y + self.f(20.0), sec.label(), col, 0.0);
            self.clicks.push((rr, Click::SettingsNav(i)));
        }

        match section {
            SettingsSection::General => self.render_settings_general(app, dl, atlas, &content),
            SettingsSection::Secrets => self.render_settings_secrets(app, dl, atlas, &content),
            SettingsSection::Variables => self.render_settings_variables(app, dl, atlas, &content),
            SettingsSection::DeployKeys => self.render_settings_deploy_keys(app, dl, atlas, &content),
            SettingsSection::Collaborators => self.render_settings_collaborators(app, dl, atlas, &content),
            SettingsSection::Webhooks => self.render_settings_webhooks(app, dl, atlas, &content),
        }
    }
}
