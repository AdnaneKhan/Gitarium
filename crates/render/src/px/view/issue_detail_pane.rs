//! The issue/PR detail view: a fixed title bar (PR approve/merge chips + an
//! in-page search box) over the content. Issues are one scrollable markdown
//! body; PRs split into a left column (prose) and a right column (checks /
//! reviews / mergeability), each scrolled on its own.

use super::issue_detail_body::{detail_left_rows, detail_right_rows, detail_rows};
use super::md::{row_text, MdRow, MdSizes};
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
        let body_w = panel.w - self.f(32.0);

        let (number, is_pr, title, state, method, busy, searching) = {
            let Some(d) = app.rv.as_ref().and_then(|rv| rv.detail.as_ref()) else { return };
            (d.number, d.is_pr, d.title.clone(), d.state.clone(), d.merge_method, d.action_busy, d.search.is_some())
        };
        let can_approve = app.token.is_some();
        let can_merge = app.can_edit_repo();

        // --- title bar: back (+ search) on the left, PR action chips right.
        let bx = self.tool_chip(dl, atlas, "‹ BACK", panel.x + self.f(10.0), top + self.f(8.0), CYAN, Click::DetailBack, wid(Z_DETAIL, 0));
        if !searching {
            self.tool_chip(dl, atlas, "/ SEARCH", bx, top + self.f(8.0), MAGENTA, Click::DetailSearchOpen, wid(Z_DETAIL, 6));
        }
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
        dl.solid(RectF::new(body_x, top + self.f(62.0), body_w, 1.0), BORDER);

        // The search box, when open, sits between the title bar and the body.
        let mut body_top = top + self.f(70.0);
        if searching {
            self.detail_search_row(app, dl, atlas, body_x, body_w, top + self.f(68.0));
            body_top += self.f(34.0);
        }
        let body_h = panel.bottom() - body_top - self.f(10.0);

        if is_pr {
            // Two columns: prose left, merge requirements right.
            let gap = self.f(24.0);
            let left_w = ((body_w - gap) * 0.60).round();
            let left = RectF::new(body_x, body_top, left_w, body_h);
            let right_x = left.right() + gap;
            let right = RectF::new(right_x, body_top, body_x + body_w - right_x, body_h);
            dl.solid(RectF::new((left.right() + right.x) / 2.0, body_top, 1.0, body_h), BORDER);
            dl.text(atlas, UI, self.f(11.0), right.x, body_top - self.f(2.0), "CHECKS · REVIEWS", DIM, self.f(2.0));

            let (lrows, lurls) = {
                let Some(d) = app.rv.as_ref().and_then(|rv| rv.detail.as_ref()) else { return };
                detail_left_rows(d, left.w - self.f(10.0), &sizes, atlas)
            };
            let rrows = {
                let Some(d) = app.rv.as_ref().and_then(|rv| rv.detail.as_ref()) else { return };
                detail_right_rows(d, &sizes)
            };
            app.layout.detail_h = (left.h / row_h).max(1.0) as usize;
            self.detail_column(app, dl, atlas, &lrows, &lurls, left, row_h, false);
            self.detail_column(app, dl, atlas, &rrows, &[], right, row_h, true);
        } else {
            let body = RectF::new(body_x, body_top, body_w, body_h);
            let (rows, urls) = {
                let Some(d) = app.rv.as_ref().and_then(|rv| rv.detail.as_ref()) else { return };
                detail_rows(d, body.w - self.f(10.0), &sizes, atlas)
            };
            app.layout.detail_h = (body.h / row_h).max(1.0) as usize;
            self.detail_column(app, dl, atlas, &rows, &urls, body, row_h, false);
        }
    }

    /// Draw one scrollable markdown column. `meta` selects the right column's
    /// scroll state; the left column also gets search match bands.
    #[allow(clippy::too_many_arguments)]
    fn detail_column(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, rows: &[MdRow], urls: &[String], area: RectF, row_h: f32, meta: bool) {
        // Search runs over the left/prose column only; it may scroll to a match.
        let (matches, cur) = if meta { (Vec::new(), None) } else { search_matches(app, rows) };
        let total = rows.len();
        let max = (total as f32 * row_h - area.h).max(0.0);
        let max_rows = (max / row_h).ceil() as usize;
        let scroll_rows = match app.rv.as_mut().and_then(|rv| rv.detail.as_mut()) {
            Some(d) => {
                let cur = if meta { &mut d.meta_scroll } else { &mut d.scroll };
                *cur = (*cur).min(max_rows);
                *cur
            }
            None => 0,
        };
        let kind = if meta { Scroll::DetailMeta } else { Scroll::Detail };
        let s = self.scrolls.entry(skey(kind)).or_insert_with(|| Smooth::new(0.0));
        s.target = (scroll_rows as f32 * row_h).clamp(0.0, max);
        if s.tick(self.dt, 14.0) {
            self.active = true;
        }
        let offset = s.v.clamp(0.0, max);
        match_bands(dl, &matches, cur, area, row_h, offset);
        self.draw_markdown(dl, atlas, rows, urls, area, row_h, offset);
        self.scrollbar(dl, &area, total as f32 * row_h, offset);
        self.wheels.push((area, kind, row_h, max));
    }

    /// The in-page search box: input field, prev/next, "n/m" count, close.
    fn detail_search_row(&mut self, app: &App, dl: &mut DrawList, atlas: &mut Atlas, x0: f32, w: f32, y: f32) {
        let Some(s) = app.rv.as_ref().and_then(|rv| rv.detail.as_ref()).and_then(|d| d.search.as_ref()) else { return };
        let input = s.query.clone_shallow();
        let count = if s.matches.is_empty() { "0/0".to_string() } else { format!("{}/{}", s.idx + 1, s.matches.len()) };
        let cy = y + self.f(15.0);
        let mut rx = self.chip(dl, atlas, "✕", x0 + w, cy, DIM, Click::DetailSearchClose, wid(Z_DETAIL, 10));
        rx = self.chip(dl, atlas, "›", rx, cy, CYAN, Click::DetailSearchNext, wid(Z_DETAIL, 11));
        rx = self.chip(dl, atlas, "‹", rx, cy, CYAN, Click::DetailSearchPrev, wid(Z_DETAIL, 12));
        let cw = dl.text_width(atlas, MONO, self.f(11.0), &count, 0.0);
        rx -= cw + self.f(10.0);
        dl.text(atlas, MONO, self.f(11.0), rx, cy + self.f(4.0), &count, FAINT, 0.0);
        let field = RectF::new(x0, y, (rx - x0 - self.f(10.0)).max(self.f(40.0)), self.f(28.0));
        self.input_field(dl, atlas, &input, field, true);
    }
}

/// Recompute search-match rows onto the detail (jump to first on change).
fn search_matches(app: &mut App, rows: &[MdRow]) -> (Vec<usize>, Option<usize>) {
    let Some(d) = app.rv.as_mut().and_then(|rv| rv.detail.as_mut()) else { return (Vec::new(), None) };
    let q = match &d.search {
        Some(s) => s.query.text.trim().to_lowercase(),
        None => return (Vec::new(), None),
    };
    let new: Vec<usize> = if q.is_empty() {
        Vec::new()
    } else {
        rows.iter().enumerate().filter(|(_, r)| row_text(r).to_lowercase().contains(&q)).map(|(i, _)| i).collect()
    };
    let changed = d.search.as_ref().map(|s| s.matches != new).unwrap_or(false);
    let first = new.first().copied();
    if changed {
        if let Some(s) = d.search.as_mut() {
            s.matches = new;
            s.idx = 0;
        }
        if let Some(m) = first {
            d.scroll = m.saturating_sub(2);
        }
    }
    let s = d.search.as_ref().unwrap();
    (s.matches.clone(), s.matches.get(s.idx).copied())
}

/// Faint band behind each visible matching row (stronger for the current).
fn match_bands(dl: &mut DrawList, matches: &[usize], cur: Option<usize>, area: RectF, row_h: f32, offset: f32) {
    if matches.is_empty() {
        return;
    }
    dl.push_clip(area);
    for &m in matches {
        let y = area.y + m as f32 * row_h - offset;
        if y + row_h < area.y || y > area.bottom() {
            continue;
        }
        let a = if Some(m) == cur { 0.30 } else { 0.14 };
        dl.solid(RectF::new(area.x, y, area.w, row_h), with_a(YELLOW, a));
    }
    dl.pop_clip();
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
