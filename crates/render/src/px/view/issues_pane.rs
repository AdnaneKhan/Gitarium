//! The Issues / Pulls list tab: a full-width list of the 100 most
//! recently-updated open issues or PRs. The per-row detail lives in
//! `issue_detail_pane.rs`.

use super::*;

/// One list row, normalized from either an `Issue` or a `Pull` so the draw
/// loop is shared.
struct Row {
    number: u64,
    title: String,
    author: String,
    age: String,
    open: bool,
    draft: bool,
    comments: i64,
    /// (name, hex color) per label.
    labels: Vec<(String, String)>,
}

/// GitHub label hex ("d73a4a") → px color; falls back to a neutral tone.
pub(super) fn label_color(hex: &str) -> Color {
    let h = hex.trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&h[0..2], 16),
            u8::from_str_radix(&h[2..4], 16),
            u8::from_str_radix(&h[4..6], 16),
        ) {
            return crate::px::theme::c(crate::ui::grid::Rgb(r, g, b), 1.0);
        }
    }
    DIM
}

enum St {
    Note(String, bool),
    Rows(Vec<Row>),
}

impl View {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn issues_tab(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, top: f32, bottom: f32, is_pulls: bool) {
        let panel = RectF::new(self.f(16.0), top, w - self.f(44.0), bottom - top);
        self.panel(dl, panel);
        let kind = if is_pulls { "PULL REQUESTS · RECENTLY UPDATED" } else { "ISSUES · RECENTLY UPDATED" };
        dl.text(atlas, UI, self.f(12.0), panel.x + self.f(14.0), top + self.f(24.0), kind, DIM, self.f(2.5));

        let row_h = self.f(38.0);
        let list = RectF::new(panel.x + self.f(8.0), top + self.f(36.0), panel.w - self.f(16.0), panel.h - self.f(46.0));
        app.layout.issues_h = (list.h / row_h).max(1.0) as usize;

        let rv = app.rv.as_ref().unwrap();
        let st = if is_pulls {
            match &rv.pulls {
                Loadable::Loading | Loadable::Idle => St::Note("FETCHING PULL REQUESTS…".into(), false),
                Loadable::Failed(e) => St::Note(e.clone(), true),
                Loadable::Ready(v) if v.is_empty() => St::Note("NO OPEN PULL REQUESTS".into(), false),
                Loadable::Ready(v) => St::Rows(v.iter().map(|p| Row {
                    number: p.number,
                    title: p.title.clone(),
                    author: p.user.as_ref().map(|u| u.login.clone()).unwrap_or_default(),
                    age: crate::app::fmt_age(&p.updated_at),
                    open: !p.merged && p.state == "open",
                    draft: p.draft,
                    comments: p.comments,
                    labels: p.labels.iter().map(|l| (l.name.clone(), l.color.clone())).collect(),
                }).collect()),
            }
        } else {
            match &rv.issues {
                Loadable::Loading | Loadable::Idle => St::Note("FETCHING ISSUES…".into(), false),
                Loadable::Failed(e) => St::Note(e.clone(), true),
                Loadable::Ready(v) if v.is_empty() => St::Note("NO OPEN ISSUES".into(), false),
                Loadable::Ready(v) => St::Rows(v.iter().map(|i| Row {
                    number: i.number,
                    title: i.title.clone(),
                    author: i.user.as_ref().map(|u| u.login.clone()).unwrap_or_default(),
                    age: crate::app::fmt_age(&i.updated_at),
                    open: i.state == "open",
                    draft: false,
                    comments: i.comments,
                    labels: i.labels.iter().map(|l| (l.name.clone(), l.color.clone())).collect(),
                }).collect()),
            }
        };

        let rows = match st {
            St::Note(msg, err) => {
                if err {
                    let m = dl.fit(atlas, UI, self.f(13.0), &msg, list.w);
                    dl.text(atlas, UI, self.f(13.0), list.x + self.f(6.0), list.y + self.f(20.0), &m, RED, 0.0);
                } else {
                    self.sweep_note(dl, atlas, list.x + self.f(6.0), list.y + self.f(20.0), list.w, &msg);
                }
                return;
            }
            St::Rows(r) => r,
        };

        let count = rows.len();
        let sel = (if is_pulls { app.rv.as_ref().unwrap().pulls_sel } else { app.rv.as_ref().unwrap().issues_sel })
            .min(count.saturating_sub(1));
        let offset = self.list_scroll(Scroll::Issues, Z_ISSUE, sel, count, row_h, list.h);
        dl.push_clip(list);
        let first = (offset / row_h) as usize;
        for vis in 0..(list.h / row_h) as usize + 2 {
            let i = first + vis;
            if i >= count {
                break;
            }
            let row = &rows[i];
            let y = list.y + i as f32 * row_h - offset;
            let rr = RectF::new(list.x, y, list.w, row_h - 3.0);
            let hv = self.hover_amt(wid(Z_ISSUE, i), rr.contains(self.hot.0, self.hot.1));
            let a = if i == sel { 0.12 } else { 0.05 * hv };
            if a > 0.005 {
                dl.rrect(rr, self.f(3.0), with_a(CYAN, a), 1.0);
            }
            let baseline = y + self.f(22.0);
            let (icon, ic) = if row.draft {
                ('◌', DIM)
            } else if row.open {
                ('●', GREEN)
            } else {
                ('●', with_a(MAGENTA, 0.8))
            };
            dl.text(atlas, MONO, self.f(11.0), rr.x + self.f(8.0), baseline, &icon.to_string(), ic, 0.0);
            let num = format!("#{}", row.number);
            let x = dl.text(atlas, MONO, self.f(12.0), rr.x + self.f(24.0), baseline, &num, with_a(CYAN, 0.7), 0.0);
            let meta = if row.comments > 0 {
                format!("[{}]  @{} · {}", row.comments, row.author, row.age)
            } else {
                format!("@{} · {}", row.author, row.age)
            };
            let mw = dl.text_width(atlas, UI, self.f(11.0), &meta, 0.0);
            let title_start = x + self.f(12.0);
            let avail = (rr.right() - title_start - mw - self.f(16.0)).max(self.f(60.0));
            // Reserve part of the row for labels, the rest for the title.
            let label_budget = if row.labels.is_empty() { 0.0 } else { (avail * 0.45).min(self.f(240.0)) };
            let title = dl.fit(atlas, UI, self.f(13.5), &row.title, avail - label_budget);
            let tw = dl.text_width(atlas, UI, self.f(13.5), &title, 0.0);
            dl.text(atlas, UI, self.f(13.5), title_start, baseline, &title, TEXT, 0.0);
            let lend = rr.right() - mw - self.f(12.0);
            let mut lx = title_start + tw + self.f(10.0);
            for (name, hex) in &row.labels {
                lx = self.label_pill(dl, atlas, name, hex, lx, y, row_h - 3.0, lend);
            }
            dl.text(atlas, UI, self.f(11.0), rr.right() - mw - self.f(8.0), baseline, &meta, DIM, 0.0);
            self.clicks.push((rr, Click::IssueRow(i)));
        }
        dl.pop_clip();
        self.scrollbar(dl, &list, count as f32 * row_h, offset);
        self.wheels.push((panel, Scroll::Issues, row_h, (count as f32 * row_h - list.h).max(0.0)));
    }

    /// Draw a compact label pill at `x`, vertically centered in a `row_h`-tall
    /// row at `row_y`; returns the next x (unchanged if it wouldn't fit).
    #[allow(clippy::too_many_arguments)]
    fn label_pill(&self, dl: &mut DrawList, atlas: &mut Atlas, name: &str, hex: &str, x: f32, row_y: f32, row_h: f32, max_x: f32) -> f32 {
        if x >= max_x {
            return x;
        }
        let col = label_color(hex);
        let px = self.f(10.5);
        let shown: String = if name.chars().count() > 16 {
            format!("{}…", name.chars().take(15).collect::<String>())
        } else {
            name.to_string()
        };
        let tw = dl.text_width(atlas, UI_BOLD, px, &shown, self.f(0.5));
        let pw = tw + self.f(12.0);
        if x + pw > max_x {
            return x;
        }
        let ph = self.f(16.0);
        let r = RectF::new(x, row_y + (row_h - ph) / 2.0, pw, ph);
        dl.rrect(r, self.f(3.0), with_a(col, 0.18), 1.0);
        dl.border(r, self.f(3.0), 1.0, with_a(col, 0.7));
        let (asc, lh) = atlas.metrics(UI_BOLD, px);
        dl.text(atlas, UI_BOLD, px, r.x + self.f(6.0), r.y + (ph - lh) / 2.0 + asc, &shown, readable(col), self.f(0.5));
        r.right() + self.f(5.0)
    }
}

/// Lift very dark label colors toward white so the name stays legible on the
/// dark HUD; bright labels are used as-is.
pub(super) fn readable(c: Color) -> Color {
    let lum = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
    if lum >= 0.5 {
        c
    } else {
        let t = (0.62 - lum).clamp(0.0, 1.0);
        [c[0] + (1.0 - c[0]) * t, c[1] + (1.0 - c[1]) * t, c[2] + (1.0 - c[2]) * t, c[3]]
    }
}
