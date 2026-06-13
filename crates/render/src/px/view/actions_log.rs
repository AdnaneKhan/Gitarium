//! The drilled-in job-log view: header chips (back / download / search), the
//! in-log search box with match highlighting and jumping, click-drag text
//! selection, and a scrollbar. Logs render raw (timestamps kept).

use super::*;

/// Char-index ranges of `q` (already lowercased) within `line_lower`.
fn match_cols(line_lower: &str, q: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if q.is_empty() {
        return out;
    }
    let qn = q.chars().count();
    let mut start = 0;
    while let Some(bpos) = line_lower[start..].find(q) {
        let b = start + bpos;
        let c0 = line_lower[..b].chars().count();
        out.push((c0, c0 + qn));
        start = b + q.len();
    }
    out
}

impl View {
    pub(super) fn job_log_view(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, right: RectF, jlist: RectF) {
        // Snapshot what we need so the app borrow doesn't span the &mut self draws.
        let rv = app.rv.as_ref().unwrap();
        let (job_id, state) = rv.job_logs.as_ref().unwrap();
        let job = rv.jobs.as_ref().and_then(|(_, l)| l.ready()).and_then(|js| js.iter().find(|j| j.id == *job_id));
        let name = job.map(|j| j.name.clone()).unwrap_or_else(|| format!("job {}", job_id));
        let ready = matches!(state, Loadable::Ready(_));
        let fail = match state {
            Loadable::Failed(e) => Some((e.clone(), job.and_then(|j| j.html_url.clone()))),
            _ => None,
        };
        let loading = matches!(state, Loadable::Loading | Loadable::Idle);
        let searching = rv.log_search.is_some();

        // Header: job name + right-aligned chips.
        let title = dl.fit(atlas, UI_BOLD, self.f(13.0), &name, jlist.w * 0.45);
        dl.text(atlas, UI_BOLD, self.f(13.0), jlist.x + self.f(4.0), jlist.y + self.f(18.0), &title, TEXT, 0.0);
        let cy = jlist.y + self.f(12.0);
        let mut rx = self.chip(dl, atlas, "‹ BACK", right.right() - self.f(14.0), cy, CYAN, Click::JobLogBack, wid(Z_RUN, 2000));
        if ready {
            rx = self.chip(dl, atlas, "DOWNLOAD", rx, cy, GREEN, Click::DownloadLog, wid(Z_RUN, 2001));
            if !searching {
                rx = self.chip(dl, atlas, "SEARCH", rx, cy, MAGENTA, Click::LogSearchOpen, wid(Z_RUN, 2002));
            }
        }
        let _ = rx;

        let mut top = jlist.y + self.f(30.0);
        if searching {
            self.log_search_row(app, dl, atlas, jlist, top);
            top += self.f(34.0);
        }
        let area = RectF::new(jlist.x, top, jlist.w, jlist.bottom() - top);

        if loading {
            self.sweep_note(dl, atlas, area.x + self.f(6.0), area.y + self.f(20.0), area.w, "FETCHING LOGS…");
        } else if let Some((e, url)) = fail {
            let m = dl.fit(atlas, UI, self.f(13.0), &format!("couldn't load logs: {}", e), area.w);
            dl.text(atlas, UI, self.f(13.0), area.x + self.f(6.0), area.y + self.f(20.0), &m, RED, 0.0);
            if let Some(url) = url {
                let label = "open job on GitHub ↗";
                let lw = dl.text_width(atlas, UI, self.f(13.0), label, 0.0);
                let lr = RectF::new(area.x + self.f(6.0), area.y + self.f(40.0), lw, self.f(20.0));
                dl.text(atlas, UI, self.f(13.0), lr.x, area.y + self.f(54.0), label, CYAN, 0.0);
                let i = self.link_urls.len();
                self.link_urls.push(url);
                self.clicks.push((lr, Click::OpenUrl(i)));
            }
        } else {
            self.draw_log_body(app, dl, atlas, area);
        }
    }

    /// The search box row: input field, prev/next, "n/m" count, close.
    fn log_search_row(&mut self, app: &App, dl: &mut DrawList, atlas: &mut Atlas, jlist: RectF, y: f32) {
        let rv = app.rv.as_ref().unwrap();
        let Some(s) = rv.log_search.as_ref() else { return };
        let input = s.query.clone_shallow();
        let count = if s.matches.is_empty() {
            "0/0".to_string()
        } else {
            format!("{}/{}", s.idx + 1, s.matches.len())
        };
        let cy = y + self.f(15.0);
        let mut rx = self.chip(dl, atlas, "✕", jlist.right() - self.f(2.0), cy, DIM, Click::LogSearchClose, wid(Z_RUN, 2010));
        rx = self.chip(dl, atlas, "›", rx, cy, CYAN, Click::LogSearchNext, wid(Z_RUN, 2011));
        rx = self.chip(dl, atlas, "‹", rx, cy, CYAN, Click::LogSearchPrev, wid(Z_RUN, 2012));
        let cw = dl.text_width(atlas, MONO, self.f(11.0), &count, 0.0);
        rx -= cw + self.f(10.0);
        dl.text(atlas, MONO, self.f(11.0), rx, cy + self.f(4.0), &count, FAINT, 0.0);
        let field = RectF::new(jlist.x + self.f(2.0), y, rx - jlist.x - self.f(10.0), self.f(28.0));
        self.input_field(dl, atlas, &input, field, true);
    }

    fn draw_log_body(&mut self, app: &App, dl: &mut DrawList, atlas: &mut Atlas, area: RectF) {
        let rv = app.rv.as_ref().unwrap();
        let text = match &rv.job_logs {
            Some((_, Loadable::Ready(t))) => t,
            _ => return,
        };
        let lines: Vec<&str> = text.lines().collect();
        let lh = self.f(15.0);
        let fs = self.f(11.0);
        let adv = atlas.advance(MONO, fs, 'M').max(1.0);
        let scroll = rv.jobs_scroll.min(lines.len().saturating_sub(1));
        self.log_geom = Some(LogGeom { area, lh, adv, scroll, lines: lines.len() });

        let sel = self.log_sel.map(|(a, b)| if a <= b { (a, b) } else { (b, a) });
        let query = rv.log_search.as_ref().map(|s| s.query.text.trim().to_lowercase()).unwrap_or_default();
        let cur_line = rv.log_search.as_ref().and_then(|s| s.matches.get(s.idx).copied());

        dl.push_clip(area);
        for (vis, li) in (scroll..lines.len()).enumerate() {
            let y = area.y + self.f(12.0) + vis as f32 * lh;
            if y > area.bottom() {
                break;
            }
            let line = lines[li];
            let nchars = line.chars().count();
            let rowtop = y - self.f(11.0);
            // Selection background.
            if let Some((a, b)) = sel {
                if li >= a.0 && li <= b.0 {
                    let c0 = if li == a.0 { a.1.min(nchars) } else { 0 };
                    let c1 = if li == b.0 { b.1.min(nchars) } else { nchars };
                    if c1 > c0 {
                        let x = area.x + self.f(4.0) + c0 as f32 * adv;
                        dl.solid(RectF::new(x, rowtop, (c1 - c0) as f32 * adv, lh), with_a(CYAN, 0.22));
                    }
                }
            }
            // Search-match background (stronger for the current match).
            if !query.is_empty() {
                let ll = line.to_lowercase();
                let strong = cur_line == Some(li);
                for (c0, c1) in match_cols(&ll, &query) {
                    let x = area.x + self.f(4.0) + c0 as f32 * adv;
                    let col = with_a(YELLOW, if strong { 0.55 } else { 0.28 });
                    dl.solid(RectF::new(x, rowtop, (c1 - c0) as f32 * adv, lh), col);
                }
            }
            let fitted = dl.fit(atlas, MONO, fs, line, area.w - self.f(10.0));
            dl.text(atlas, MONO, fs, area.x + self.f(4.0), y, &fitted, with_a(TEXT, 0.85), 0.0);
        }
        dl.pop_clip();
        self.scrollbar(dl, &area, lines.len() as f32 * lh, scroll as f32 * lh);
        self.wheels.push((area, Scroll::Jobs, lh, (lines.len() as f32 * lh - area.h).max(0.0)));
    }
}
