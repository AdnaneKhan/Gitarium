//! The Agent route: toolbar, transcript orchestration, prompt input,
//! and the key/endpoint entry panel.

use super::agent_text::build_transcript;
use super::*;

impl View {
    pub(super) fn agent_screen(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, yoff: f32) {
        let hh = self.header(app, dl, atlas, w, "AGENT", None);


        // No API key yet: key/endpoint entry panel, mirroring the auth screen.
        if app.anthropic_key.is_none() {
            self.agent_key_prompt(app, dl, atlas, w, h, yoff);
            return;
        }
        let top = hh + self.f(8.0) + yoff;
        let bottom = h - self.f(34.0);

        // Toolbar: provider endpoint + busy pulse left, action chips right.
        // The model id is intentionally not shown.
        let cy = top + self.f(14.0);
        let mut right = w - self.f(16.0);
        right = self.chip(dl, atlas, "CLEAR", right, cy, CYAN, Click::AgentClear, wid(Z_CHIP, 40));
        right = self.chip(dl, atlas, "KEY", right, cy, MAGENTA, Click::AgentResetKey, wid(Z_CHIP, 41));
        let _ = self.chip(dl, atlas, "MODEL", right, cy, GREEN, Click::ModelPickBtn, wid(Z_CHIP, 42));
        let mut mx = self.f(18.0);
        if let Some(u) = &app.anthropic_url {
            let host = u.trim_start_matches("https://").trim_start_matches("http://");
            mx = dl.text(atlas, MONO, self.f(11.5), mx, cy + self.f(4.0), host, DIM, 0.0);
        }
        if app.agent.busy {
            let a = 0.55 + 0.45 * ((self.time * 0.006).sin() as f32);
            dl.text(atlas, UI, self.f(12.0), mx + self.f(18.0), cy + self.f(4.0), "WORKING…", with_a(YELLOW, a), self.f(2.0));
            self.active = true;
        }

        let pane = RectF::new(self.f(16.0), top + self.f(30.0), w - self.f(32.0), bottom - top - self.f(80.0));
        self.panel(dl, pane);

        // ---- transcript, wrapped to mono columns ----
        let pad = self.f(14.0);
        let inner = RectF::new(pane.x + pad, pane.y + pad, pane.w - pad * 2.0, pane.h - pad * 2.0);
        let px = self.f(12.5);
        let (asc, lh) = atlas.metrics(MONO, px);
        let adv = atlas.advance(MONO, px, 'M').max(1.0);
        let cols = ((inner.w / adv) as usize).max(8);
        let row = lh * 1.4;

        let (lines, urls) = build_transcript(&app.agent.transcript, cols);
        self.link_urls = urls;
        if lines.is_empty() {
            dl.text(
                atlas,
                UI,
                self.f(14.5),
                inner.x,
                inner.y + self.f(20.0),
                "DESCRIBE A GITHUB TASK — THE AGENT PLANS AND RUNS THE API CALLS",
                DIM,
                self.f(1.5),
            );
            dl.text(
                atlas,
                UI,
                self.f(13.0),
                inner.x,
                inner.y + self.f(46.0),
                "try: list my open PRs · open an issue here · what changed in the last release?",
                FAINT,
                0.0,
            );
        }

        // A (line, col) selection silently re-targets different text when
        // wrapping shifts (streaming re-wrap, resize): keep it only while
        // every wrapped line up to its end is unchanged.
        if let Some((a, b)) = self.agent_sel {
            let hi = a.0.max(b.0);
            let intact = hi < lines.len()
                && hi < self.agent_lines.len()
                && (0..=hi).all(|i| lines[i].text == self.agent_lines[i]);
            if !intact {
                self.agent_sel = None;
            }
        }

        let content_h = lines.len() as f32 * row;
        let max = (content_h - inner.h).max(0.0);
        let rev_changed = app.agent.rev != self.last_agent_rev;
        self.last_agent_rev = app.agent.rev;
        let s = self.scrolls.entry(skey(Scroll::Agent)).or_insert_with(|| Smooth::new(0.0));
        // Re-stick to the bottom on new content only when the user was
        // already reading the bottom (against the *previous* extent) and
        // isn't mid-drag-selection; never yank them off history.
        if rev_changed && self.drag != Drag::Agent && s.target >= self.last_agent_max - row {
            s.target = max;
        }
        self.last_agent_max = max;
        s.target = s.target.clamp(0.0, max);
        if s.tick(self.dt, 14.0) {
            self.active = true;
        }
        let offset = s.v.clamp(0.0, max);

        // Expose layout + text to the input layer (drag selection, copy).
        self.agent_lines = lines.iter().map(|l| l.text.clone()).collect();
        self.agent_src = lines.iter().map(|l| l.src).collect();
        self.agent_xs = lines
            .iter()
            .map(|l| {
                if l.label {
                    // Mirrors the label draw call: UI_BOLD at 11.5 with tracking.
                    Some(atlas.char_xs(UI_BOLD, self.f(11.5), &l.text, self.f(2.5)))
                } else if l.text.is_ascii() {
                    None // uniform mono cells
                } else {
                    Some(atlas.char_xs(MONO, px, &l.text, 0.0))
                }
            })
            .collect();
        self.agent_geom = Some((inner, row, adv, offset));
        // While dragging, re-derive the head from the cursor through *this*
        // frame's offset so the band can't lag the scroll animation.
        if self.drag == Drag::Agent && self.agent_sel.is_some() {
            if let Some(pos) = self.agent_pos_at(self.mouse.0, self.mouse.1, true) {
                if let Some(sel) = &mut self.agent_sel {
                    sel.1 = pos;
                }
            }
        }
        let sel = self.agent_sel.map(|(a, b)| if a <= b { (a, b) } else { (b, a) });

        self.draw_transcript(dl, atlas, &lines, sel, inner, row, asc, adv, px, offset);
        self.scrollbar(dl, &pane, content_h + pad * 2.0, offset);
        self.wheels.push((pane, Scroll::Agent, row, max));

        // ---- prompt input ----
        let bar = RectF::new(pane.x, pane.bottom() + self.f(10.0), pane.w, self.f(40.0));
        let input = app.agent.input.clone_shallow();
        self.input_field(dl, atlas, &input, bar, !app.agent.busy);
    }

    fn agent_key_prompt(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, yoff: f32) {
        // No transcript pane this frame — a previous visit's selection
        // must not survive into the key prompt (stale Ctrl+C copy).
        self.agent_sel = None;
        let pw = self.f(560.0).min(w - self.f(32.0));
        let ph = self.f(330.0);
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 - self.f(30.0) + yoff, pw, ph);
        dl.glow(r, self.f(4.0), with_a(MAGENTA, 0.05), self.f(36.0));
        self.panel(dl, r);
        let x = r.x + self.f(30.0);
        let end = dl.text(atlas, UI_BOLD, self.f(28.0), x, r.y + self.f(52.0), "AGENT", MAGENTA, self.f(6.0));
        dl.text(atlas, UI, self.f(14.0), end + self.f(6.0), r.y + self.f(52.0), "// GITHUB AUTOPILOT", CYAN, self.f(2.5));

        let url_focused = app.agent.url_focused;
        dl.text(atlas, UI, self.f(13.0), x, r.y + self.f(92.0), "ANTHROPIC API KEY", DIM, self.f(1.5));
        let key_field = RectF::new(x, r.y + self.f(104.0), r.w - self.f(60.0), self.f(42.0));
        let key_input = app.agent.key_input.clone_shallow();
        self.input_field(dl, atlas, &key_input, key_field, !url_focused);

        dl.text(
            atlas,
            UI,
            self.f(13.0),
            x,
            r.y + self.f(184.0),
            "API ENDPOINT — OPTIONAL, DEFAULTS TO api.anthropic.com",
            DIM,
            self.f(1.5),
        );
        let url_field = RectF::new(x, r.y + self.f(196.0), r.w - self.f(60.0), self.f(42.0));
        let url_input = app.agent.url_input.clone_shallow();
        self.input_field(dl, atlas, &url_input, url_field, url_focused);

        dl.text(
            atlas,
            UI,
            self.f(12.5),
            x,
            r.y + self.f(274.0),
            "stored in localStorage · [TAB] switch field · [ESC] back",
            DIM,
            0.0,
        );
    }
}
