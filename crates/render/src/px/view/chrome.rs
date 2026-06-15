//! Window chrome: the header strip, auth screen, status bar, and toasts.

use super::*;

impl View {
    /// Top strip; returns its height.
    pub(super) fn header(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, title: &str, repo: Option<String>) -> f32 {
        let hh = self.f(52.0);
        dl.solid(RectF::new(0.0, 0.0, w, hh), BG1);
        dl.solid(RectF::new(0.0, hh - 1.0, w, 1.0), BORDER_BRIGHT);

        // Pulsing status dot.
        let pulse = 0.55 + 0.45 * ((self.time * 0.003).sin() as f32);
        let dot = RectF::new(self.f(18.0), hh / 2.0 - self.f(4.0), self.f(8.0), self.f(8.0));
        dl.glow(dot, self.f(4.0), with_a(CYAN, 0.35 * pulse), self.f(8.0));
        dl.rrect(dot, self.f(4.0), with_a(CYAN, pulse), 1.0);
        self.active = true;

        let mut x = dl.text(atlas, UI_BOLD, self.f(19.0), self.f(38.0), self.f(33.0), title, TEXT, self.f(3.5));

        if let Some(branch) = repo {
            // Branch chip.
            let label = format!("{} ▾", branch);
            let px = self.f(13.0);
            let tw = dl.text_width(atlas, MONO, px, &label, 0.0);
            let chip = RectF::new(x + self.f(16.0), hh / 2.0 - self.f(13.0), tw + self.f(22.0), self.f(26.0));
            let hv = self.hover_amt(wid(Z_CHIP, 99), chip.contains(self.hot.0, self.hot.1));
            if hv > 0.01 {
                dl.glow(chip, self.f(3.0), with_a(CYAN, 0.2 * hv), self.f(8.0));
            }
            dl.rrect(chip, self.f(3.0), with_a(CYAN, 0.06 + 0.08 * hv), 1.0);
            dl.border(chip, self.f(3.0), 1.0, with_a(CYAN, 0.7));
            let (asc, lh) = atlas.metrics(MONO, px);
            dl.text(atlas, MONO, px, chip.x + self.f(11.0), chip.y + (chip.h - lh) / 2.0 + asc, &label, CYAN, 0.0);
            self.clicks.push((chip, Click::BranchBtn));
            x = chip.right();

            // Tabs. Settings only appears for viewers with write access.
            let mut tabs: Vec<(Tab, &str)> = vec![
                (Tab::Code, "CODE"),
                (Tab::Issues, "ISSUES"),
                (Tab::Pulls, "PULLS"),
                (Tab::Actions, "ACTIONS"),
            ];
            if app.can_edit_repo() {
                tabs.push((Tab::Settings, "SETTINGS"));
            }
            let sel_tab = app.rv.as_ref().map(|rv| rv.tab).unwrap_or(Tab::Code);
            let mut tx = x + self.f(30.0);
            for (i, (tab, label)) in tabs.iter().enumerate() {
                let px = self.f(14.0);
                let tw = dl.text_width(atlas, UI_BOLD, px, label, self.f(3.0));
                let region = RectF::new(tx - self.f(8.0), 0.0, tw + self.f(16.0), hh);
                let hv = self.hover_amt(wid(Z_TAB, i), region.contains(self.hot.0, self.hot.1));
                let on = *tab == sel_tab;
                let c = if on {
                    CYAN
                } else {
                    with_a(TEXT, 0.45 + 0.4 * hv)
                };
                dl.text(atlas, UI_BOLD, px, tx, self.f(32.0), label, c, self.f(3.0));
                if on {
                    self.tab_x.target = tx;
                    self.tab_w.target = tw;
                    if self.tab_x.v == 0.0 {
                        self.tab_x.snap(tx);
                        self.tab_w.snap(tw);
                    }
                }
                self.clicks.push((region, Click::Tab(*tab)));
                tx += tw + self.f(26.0);
            }
            if self.tab_x.tick(self.dt, 18.0) | self.tab_w.tick(self.dt, 18.0) {
                self.active = true;
            }
            let underline = RectF::new(self.tab_x.v, hh - self.f(3.0), self.tab_w.v, self.f(2.0));
            dl.glow(underline, 1.0, with_a(CYAN, 0.3), self.f(6.0));
            dl.solid(underline, CYAN);
        }

        // Right side: identity.
        let who = app.login.clone().unwrap_or_else(|| "ANONYMOUS".into());
        let tw = dl.text_width(atlas, UI, self.f(13.0), &who, self.f(1.5));
        dl.text(atlas, UI, self.f(13.0), w - tw - self.f(20.0), self.f(32.0), &who, DIM, self.f(1.5));
        hh
    }

    pub(super) fn auth_screen(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, yoff: f32) {
        let pw = self.f(560.0).min(w - self.f(32.0));
        let ph = self.f(264.0);
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 - self.f(30.0) + yoff, pw, ph);
        dl.glow(r, self.f(4.0), with_a(CYAN, 0.05), self.f(36.0));
        self.panel(dl, r);

        let x = r.x + self.f(30.0);
        let title_px = self.f(34.0);
        let end = dl.text(atlas, UI_BOLD, title_px, x, r.y + self.f(56.0), "GITARIUM", CYAN, self.f(7.0));
        dl.text(atlas, UI, self.f(14.0), end + self.f(6.0), r.y + self.f(56.0), "// GITHUB INTERFACE", MAGENTA, self.f(2.5));

        dl.text(
            atlas,
            UI,
            self.f(13.0),
            x,
            r.y + self.f(96.0),
            "ACCESS TOKEN — ENTER ON EMPTY FOR ANONYMOUS",
            DIM,
            self.f(1.5),
        );
        let field = RectF::new(x, r.y + self.f(112.0), r.w - self.f(60.0), self.f(42.0));
        let input = app.token_input.clone_shallow();
        self.input_field(dl, atlas, &input, field, !app.auth_busy);

        let msg_y = r.y + self.f(190.0);
        if app.auth_busy {
            let a = 0.55 + 0.45 * ((self.time * 0.006).sin() as f32);
            dl.text(atlas, UI, self.f(14.0), x, msg_y, "VALIDATING TOKEN…", with_a(YELLOW, a), self.f(1.5));
            self.active = true;
        } else if let Some(err) = &app.auth_error {
            let msg = dl.fit(atlas, UI, self.f(14.0), err, r.w - self.f(60.0));
            dl.text(atlas, UI, self.f(14.0), x, msg_y, &msg, RED, 0.0);
        } else {
            dl.text(
                atlas,
                UI,
                self.f(13.0),
                x,
                msg_y,
                "paste with Cmd/Ctrl+V · fine-grained PAT recommended",
                DIM,
                0.0,
            );
        }
    }

    pub(super) fn status_bar(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32) {
        let bh = self.f(28.0);
        let y = h - bh;
        dl.solid(RectF::new(0.0, y, w, bh), BG1);
        dl.solid(RectF::new(0.0, y, w, 1.0), BORDER_BRIGHT);
        let hints = super::hints::route_hints(app);
        let baseline = y + self.f(19.0);
        dl.text(atlas, UI, self.f(12.0), self.f(16.0), baseline, hints, with_a(DIM, 0.9), self.f(1.0));
        let rate = crate::fetch::RATE_LIMIT
            .with(|c| c.get())
            .map(|(r, l)| format!("API {}/{}", r, l))
            .unwrap_or_default();
        let tw = dl.text_width(atlas, UI, self.f(12.0), &rate, self.f(1.0));
        dl.text(atlas, UI, self.f(12.0), w - tw - self.f(16.0), baseline, &rate, DIM, self.f(1.0));
    }

    pub(super) fn toast(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32) {
        self.toast_t.target = if app.toast.is_some() { 1.0 } else { 0.0 };
        if app.toast.is_none() {
            self.toast_t.snap(0.0);
            return;
        }
        if self.toast_t.tick_n(self.dt, 13.0) {
            self.active = true;
        }
        let k = ease_out(self.toast_t.v);
        let (msg, is_err) = app.toast.clone().unwrap();
        let color = if is_err { RED } else { GREEN };
        let px = self.f(13.0);
        let tw = dl.text_width(atlas, UI, px, &msg, self.f(0.5));
        let r = RectF::new(
            w - tw - self.f(44.0) + (1.0 - k) * self.f(60.0),
            h - self.f(64.0),
            tw + self.f(28.0),
            self.f(30.0),
        );
        dl.glow(r, self.f(3.0), with_a(color, 0.25 * k), self.f(10.0));
        dl.rrect(r, self.f(3.0), BG2, 1.0);
        dl.border(r, self.f(3.0), 1.0, with_a(color, 0.9 * k));
        let (asc, lh) = atlas.metrics(UI, px);
        dl.text(atlas, UI, px, r.x + self.f(14.0), r.y + (r.h - lh) / 2.0 + asc, &msg, with_a(TEXT, k), self.f(0.5));
    }
}
