//! The issue/PR detail view: a fixed title bar with the PR action chips
//! (approve / merge / method) over a scrollable, markdown-rendered body — the
//! description and comments (via `md`) plus the PR merge requirements.

use super::issue_detail_body::detail_rows;
use super::md::MdSizes;
use super::*;

impl View {
    pub(super) fn issue_detail(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, top: f32, bottom: f32) {
        let panel = RectF::new(self.f(16.0), top, w - self.f(44.0), bottom - top);
        self.panel(dl, panel);

        let sizes = MdSizes {
            text_px: self.f(13.5),
            mono_px: self.f(12.5),
            indent: self.f(18.0),
            h_px: [self.f(16.5), self.f(15.0), self.f(14.0), self.f(13.5), self.f(13.5), self.f(13.5)],
        };
        let row_h = atlas.metrics(UI_BOLD, sizes.h_px[0]).1.max(atlas.metrics(UI, sizes.text_px).1 * 1.25).ceil();
        let body_x = panel.x + self.f(16.0);
        let body_top = top + self.f(70.0);
        let body = RectF::new(body_x, body_top, panel.w - self.f(32.0), panel.bottom() - body_top - self.f(10.0));

        // Read everything off the detail up front (owned), so the rest can
        // borrow self / app mutably for drawing and scroll clamping.
        let (number, is_pr, title, state, method, busy, rows, urls) = {
            let Some(d) = app.rv.as_ref().and_then(|rv| rv.detail.as_ref()) else { return };
            let (rows, urls) = detail_rows(d, body.w - self.f(10.0), &sizes, atlas);
            (d.number, d.is_pr, d.title.clone(), d.state.clone(), d.merge_method, d.action_busy, rows, urls)
        };
        let can_approve = app.token.is_some();
        let can_merge = app.can_edit_repo();

        // --- title bar: back + (PR) action chips, then "#n [STATE] title".
        self.tool_chip(dl, atlas, "‹ BACK", panel.x + self.f(10.0), top + self.f(8.0), CYAN, Click::DetailBack, wid(Z_DETAIL, 0));
        if is_pr {
            let cy = top + self.f(20.0);
            let mut right = panel.right() - self.f(12.0);
            if busy {
                self.chip(dl, atlas, "WORKING…", right, cy, YELLOW, Click::DetailBack, wid(Z_DETAIL, 3));
                self.active = true;
            } else {
                if can_merge {
                    right = self.chip(dl, atlas, "MERGE", right, cy, GREEN, Click::Merge, wid(Z_DETAIL, 3));
                    right = self.chip(dl, atlas, method.label(), right, cy, CYAN, Click::MergeMethodCycle, wid(Z_DETAIL, 4));
                }
                if can_approve {
                    self.chip(dl, atlas, "APPROVE", right, cy, MAGENTA, Click::Approve, wid(Z_DETAIL, 5));
                }
            }
        }
        let ty = top + self.f(52.0);
        let mut x = dl.text(atlas, MONO, self.f(13.0), body_x, ty, &format!("#{}", number), with_a(CYAN, 0.7), 0.0);
        let (scol, slab) = state_badge(&state);
        x = dl.text(atlas, UI_BOLD, self.f(11.0), x + self.f(10.0), ty, slab, scol, self.f(1.5));
        let title_fit = dl.fit(atlas, UI_BOLD, self.f(15.0), &title, (panel.right() - x - self.f(28.0)).max(self.f(80.0)));
        dl.text(atlas, UI_BOLD, self.f(15.0), x + self.f(12.0), ty, &title_fit, TEXT, 0.0);
        dl.solid(RectF::new(body_x, top + self.f(62.0), body.w, 1.0), BORDER);

        // --- scrollable, markdown-rendered body.
        let total = rows.len();
        app.layout.detail_h = (body.h / row_h).max(1.0) as usize;
        let max = (total as f32 * row_h - body.h).max(0.0);
        let max_rows = (max / row_h).ceil() as usize;
        let scroll_rows = match app.rv.as_mut().and_then(|rv| rv.detail.as_mut()) {
            Some(d) => {
                d.scroll = d.scroll.min(max_rows);
                d.scroll
            }
            None => 0,
        };
        let s = self.scrolls.entry(skey(Scroll::Detail)).or_insert_with(|| Smooth::new(0.0));
        s.target = (scroll_rows as f32 * row_h).clamp(0.0, max);
        if s.tick(self.dt, 14.0) {
            self.active = true;
        }
        let offset = s.v.clamp(0.0, max);

        self.draw_markdown(dl, atlas, &rows, &urls, body, row_h, offset);
        self.scrollbar(dl, &body, total as f32 * row_h, offset);
        self.wheels.push((body, Scroll::Detail, row_h, max));
    }
}

/// (color, label) for an issue/PR state.
fn state_badge(state: &str) -> (Color, &'static str) {
    match state {
        "open" => (GREEN, "OPEN"),
        "merged" => (MAGENTA, "MERGED"),
        "closed" => (RED, "CLOSED"),
        _ => (DIM, "—"),
    }
}
