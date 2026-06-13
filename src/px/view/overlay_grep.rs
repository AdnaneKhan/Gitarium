//! The GitHub code-search overlay: query field, result rows with the
//! matched range highlighted.

use super::*;

impl View {
    pub(super) fn ov_code_search(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, lift: f32, title: &str) {
        let Some(Overlay::CodeSearch { input, sel, searched, results, scope, more, loading_more, .. }) =
            &app.overlay
        else {
            return;
        };
        let input = input.clone_shallow();
        let sel = *sel;
        let global = *scope == SearchScope::Global;
        let armed = input.text.trim() != searched.as_str() || searched.is_empty();
        enum RState {
            Idle,
            Loading,
            Failed(String),
            // (repo, path, line, range) — repo shown only in global mode.
            Hits(Vec<(String, String, String, Option<(usize, usize)>)>),
        }
        let state = match results {
            Loadable::Idle => RState::Idle,
            Loadable::Loading => RState::Loading,
            Loadable::Failed(e) => RState::Failed(e.clone()),
            Loadable::Ready(h) => RState::Hits(
                h.iter().map(|c| (c.repo.clone(), c.path.clone(), c.line.clone(), c.range)).collect(),
            ),
        };
        let hits_len = match &state {
            RState::Hits(h) => h.len(),
            _ => 0,
        };
        // No result list to scroll (idle / loading / no hits): park the shared
        // overlay scroll at the top so a fresh search starts unscrolled.
        if hits_len == 0 {
            if let Some(s) = self.scrolls.get_mut(&skey(Scroll::Overlay)) {
                s.snap(0.0);
            }
        }
        let row_h = self.f(40.0);
        let visible = hits_len.min(8);
        let list_h = visible.max(1) as f32 * row_h;
        let pw2 = self.f(720.0).min(w - self.f(40.0));
        let ph = list_h + self.f(140.0);
        let r = RectF::new((w - pw2) / 2.0, h * 0.12 + lift, pw2, ph);
        self.overlay_panel(dl, atlas, r, &title);
        if hits_len > 0 {
            let count = format!("{} RESULTS", hits_len);
            let cw = dl.text_width(atlas, UI, self.f(11.0), &count, self.f(1.5));
            dl.text(atlas, UI, self.f(11.0), r.right() - cw - self.f(24.0), r.y + self.f(34.0), &count, FAINT, self.f(1.5));
        }
        let field = RectF::new(r.x + self.f(24.0), r.y + self.f(48.0), r.w - self.f(48.0), self.f(38.0));
        self.input_field(dl, atlas, &input, field, true);
        let hint = if armed {
            if global {
                "[ENTER] SEARCH ANYWHERE · org: repo: language: path: QUALIFIERS · DEFAULT BRANCH"
            } else {
                "[ENTER] SEARCH · GITHUB CODE SEARCH · DEFAULT BRANCH ONLY"
            }
        } else if *loading_more {
            "[ENTER] OPEN · ↑↓ SELECT · LOADING MORE…"
        } else if *more {
            "[ENTER] OPEN · ↑↓ SELECT · ↓ AT END LOADS MORE"
        } else {
            "[ENTER] OPEN · ↑↓ SELECT · EDIT QUERY TO SEARCH AGAIN"
        };
        dl.text(atlas, UI, self.f(11.0), r.x + self.f(24.0), r.y + ph - self.f(18.0), hint, FAINT, self.f(1.5));

        let y0 = field.bottom() + self.f(12.0);
        match state {
            RState::Idle => {
                dl.text(atlas, UI, self.f(13.0), r.x + self.f(24.0), y0 + self.f(16.0), "type a query, Enter to search", FAINT, self.f(1.0));
            }
            RState::Loading => {
                self.sweep_note(dl, atlas, r.x + self.f(24.0), y0 + self.f(16.0), r.w - self.f(48.0), "SEARCHING…");
            }
            RState::Failed(e) => {
                let m = dl.fit(atlas, UI, self.f(13.0), &e, r.w - self.f(48.0));
                dl.text(atlas, UI, self.f(13.0), r.x + self.f(24.0), y0 + self.f(16.0), &m, RED, 0.0);
            }
            RState::Hits(hits) => {
                if hits.is_empty() {
                    dl.text(atlas, UI, self.f(13.0), r.x + self.f(24.0), y0 + self.f(16.0), "no results", FAINT, self.f(1.0));
                }
                let list = RectF::new(r.x + self.f(16.0), y0, r.w - self.f(32.0), list_h);
                let sel = sel.min(hits.len().saturating_sub(1));
                // Branch-picker pattern: a smooth offset that follows the
                // selection during keyboard nav and is free to wheel-scroll.
                let offset = self.list_scroll(Scroll::Overlay, Z_GREP, sel, hits.len(), row_h, list.h);
                dl.push_clip(list);
                let first = (offset / row_h) as usize;
                for vis in 0..(list.h / row_h) as usize + 2 {
                    let i = first + vis;
                    if i >= hits.len() {
                        break;
                    }
                    let (repo, path, line, range) = &hits[i];
                    let y = list.y + i as f32 * row_h - offset;
                    let rr = RectF::new(list.x, y, list.w, row_h - 4.0);
                    let hv = self.hover_amt(wid(Z_GREP, i), rr.contains(self.mouse.0, self.mouse.1));
                    let a = if i == sel { 0.13 } else { 0.06 * hv };
                    if a > 0.005 {
                        dl.rrect(rr, self.f(3.0), with_a(CYAN, a), 1.0);
                    }
                    dl.push_clip(rr);
                    // Line 1: [repo › ] path (repo magenta in global mode,
                    // dir dim, filename bright).
                    let b1 = y + self.f(15.0);
                    let (dir, name) = match path.rsplit_once('/') {
                        Some((d, n)) => (format!("{}/", d), n.to_string()),
                        None => (String::new(), path.clone()),
                    };
                    let mut x = rr.x + self.f(10.0);
                    if global && !repo.is_empty() {
                        x = dl.text(atlas, MONO, self.f(11.5), x, b1, &format!("{} · ", repo), with_a(MAGENTA, 0.9), 0.0);
                    }
                    if !dir.is_empty() {
                        x = dl.text(atlas, MONO, self.f(11.5), x, b1, &dir, FAINT, 0.0);
                    }
                    dl.text(atlas, MONO, self.f(11.5), x, b1, &name, if i == sel { CYAN } else { with_a(TEXT, 0.9) }, 0.0);
                    // Line 2: matched line with the hit highlighted.
                    let b2 = y + self.f(31.0);
                    let mut x = rr.x + self.f(10.0);
                    match range {
                        Some((cs, ce)) if *ce > *cs => {
                            let pre: String = line.chars().take(*cs).collect();
                            let hit: String = line.chars().skip(*cs).take(ce - cs).collect();
                            let post: String = line.chars().skip(*ce).collect();
                            x = dl.text(atlas, MONO, self.f(11.5), x, b2, &pre, DIM, 0.0);
                            let hw = dl.text_width(atlas, MONO, self.f(11.5), &hit, 0.0);
                            dl.solid(RectF::new(x, b2 - self.f(11.0), hw, self.f(15.0)), with_a(MAGENTA, 0.18));
                            x = dl.text(atlas, MONO, self.f(11.5), x, b2, &hit, MAGENTA, 0.0);
                            dl.text(atlas, MONO, self.f(11.5), x, b2, &post, DIM, 0.0);
                        }
                        _ => {
                            dl.text(atlas, MONO, self.f(11.5), x, b2, line, DIM, 0.0);
                        }
                    }
                    dl.pop_clip();
                    self.clicks.push((rr, Click::OverlayItem(i)));
                }
                dl.pop_clip();
                self.scrollbar(dl, &list, hits.len() as f32 * row_h, offset);
                self.wheels.push((
                    list,
                    Scroll::Overlay,
                    row_h,
                    (hits.len() as f32 * row_h - list.h).max(0.0),
                ));
            }
        }
    }
}
