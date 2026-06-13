//! List-style overlays: the branch picker and the find-file palette.

use super::*;

impl View {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn ov_branch_pick(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, pw: f32, lift: f32, title: &str) {
        let Some(Overlay::BranchPick { sel, scroll }) = &app.overlay else { return };
        let (sel, scroll) = (*sel, *scroll);
        let branches = app
            .rv
            .as_ref()
            .and_then(|rv| rv.branches.ready())
            .cloned()
            .unwrap_or_default();
        let current = app.rv.as_ref().map(|rv| rv.branch.clone()).unwrap_or_default();
        let row_h = self.f(30.0);
        let list_h = (branches.len() as f32 * row_h).min(h * 0.5);
        let ph = list_h + self.f(86.0);
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
        self.overlay_panel(dl, atlas, r, &title);
        // "+ NEW" opens the new-branch modal (also bound to the `n` key) —
        // only when the repo is writable.
        if app.can_edit_repo() {
            self.chip(dl, atlas, "+ NEW", r.right() - self.f(14.0), r.y + self.f(30.0), GREEN, Click::NewBranchBtn, wid(Z_MENU, 902));
        }
        let list = RectF::new(r.x + self.f(16.0), r.y + self.f(54.0), r.w - self.f(32.0), list_h);
        app.layout.overlay_h = (list.h / row_h).max(1.0) as usize;
        let _ = scroll;
        // Same rule as the main lists: re-anchor on the selection
        // only when it moves, so wheel scrolling is free.
        let offset = self.list_scroll(Scroll::Overlay, Z_OVER, sel, branches.len(), row_h, list.h);
        dl.push_clip(list);
        let first = (offset / row_h) as usize;
        for vis in 0..(list.h / row_h) as usize + 2 {
            let i = first + vis;
            if i >= branches.len() {
                break;
            }
            let y = list.y + i as f32 * row_h - offset;
            let rr = RectF::new(list.x, y, list.w, row_h - 2.0);
            let hv = self.hover_amt(wid(Z_OVER, i), rr.contains(self.mouse.0, self.mouse.1));
            let a = if i == sel { 0.13 } else { 0.06 * hv };
            if a > 0.005 {
                dl.rrect(rr, self.f(3.0), with_a(CYAN, a), 1.0);
            }
            let baseline = y + self.f(20.0);
            if branches[i].name == current {
                dl.text(atlas, MONO, self.f(12.0), rr.x + self.f(8.0), baseline, "●", GREEN, 0.0);
            }
            let name = dl.fit(atlas, MONO, self.f(13.0), &branches[i].name, rr.w - self.f(40.0));
            dl.text(atlas, MONO, self.f(13.0), rr.x + self.f(26.0), baseline, &name, TEXT, 0.0);
            self.clicks.push((rr, Click::OverlayItem(i)));
        }
        dl.pop_clip();
        self.scrollbar(dl, &list, branches.len() as f32 * row_h, offset);
        self.wheels.push((list, Scroll::Overlay, row_h, (branches.len() as f32 * row_h - list.h).max(0.0)));
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn ov_model_pick(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, pw: f32, lift: f32, title: &str) {
        let Some(Overlay::ModelPick { models, sel }) = &app.overlay else { return };
        let sel = *sel;
        enum St {
            Loading,
            Failed(String),
            Ready(Vec<(String, String)>), // (display, id)
        }
        let st = match models {
            Loadable::Loading | Loadable::Idle => St::Loading,
            Loadable::Failed(e) => St::Failed(e.clone()),
            Loadable::Ready(m) => St::Ready(m.iter().map(|x| (x.display.clone(), x.id.clone())).collect()),
        };
        let rows = if let St::Ready(m) = &st { m.len() } else { 1 };
        let row_h = self.f(30.0);
        let list_h = (rows as f32 * row_h).clamp(row_h, h * 0.5);
        let ph = list_h + self.f(86.0);
        let r = RectF::new((w - pw) / 2.0, (h - ph) / 2.0 + lift, pw, ph);
        self.overlay_panel(dl, atlas, r, title);
        let list = RectF::new(r.x + self.f(16.0), r.y + self.f(54.0), r.w - self.f(32.0), list_h);
        match st {
            St::Loading => self.sweep_note(dl, atlas, list.x + self.f(6.0), list.y + self.f(22.0), list.w, "FETCHING MODELS…"),
            St::Failed(e) => {
                let m = dl.fit(atlas, UI, self.f(13.0), &format!("couldn't list models: {}", e), list.w - self.f(8.0));
                dl.text(atlas, UI, self.f(13.0), list.x + self.f(6.0), list.y + self.f(22.0), &m, RED, 0.0);
            }
            St::Ready(items) => {
                app.layout.overlay_h = (list.h / row_h).max(1.0) as usize;
                let sel = sel.min(items.len().saturating_sub(1));
                let offset = self.list_scroll(Scroll::Overlay, Z_OVER, sel, items.len(), row_h, list.h);
                dl.push_clip(list);
                let first = (offset / row_h) as usize;
                for vis in 0..(list.h / row_h) as usize + 2 {
                    let i = first + vis;
                    if i >= items.len() {
                        break;
                    }
                    let y = list.y + i as f32 * row_h - offset;
                    let rr = RectF::new(list.x, y, list.w, row_h - 2.0);
                    let hv = self.hover_amt(wid(Z_OVER, i), rr.contains(self.mouse.0, self.mouse.1));
                    let a = if i == sel { 0.13 } else { 0.06 * hv };
                    if a > 0.005 {
                        dl.rrect(rr, self.f(3.0), with_a(CYAN, a), 1.0);
                    }
                    let baseline = y + self.f(20.0);
                    let (display, id) = &items[i];
                    let name = dl.fit(atlas, UI, self.f(13.5), display, rr.w - self.f(20.0));
                    let nx = dl.text(atlas, UI, self.f(13.5), rr.x + self.f(10.0), baseline, &name, if i == sel { CYAN } else { TEXT }, 0.0);
                    // Show the raw id dimmed only when it differs from the name.
                    if id != display {
                        dl.text(atlas, MONO, self.f(10.5), nx + self.f(10.0), baseline, id, FAINT, 0.0);
                    }
                    self.clicks.push((rr, Click::OverlayItem(i)));
                }
                dl.pop_clip();
                self.scrollbar(dl, &list, items.len() as f32 * row_h, offset);
                self.wheels.push((list, Scroll::Overlay, row_h, (items.len() as f32 * row_h - list.h).max(0.0)));
            }
        }
    }

    pub(super) fn ov_file_search(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32, lift: f32, title: &str) {
        let Some(Overlay::FileSearch { input, sel }) = &app.overlay else { return };
        let input = input.clone_shallow();
        let sel = *sel;
        let results: Vec<String> = app
            .rv
            .as_ref()
            .and_then(|rv| rv.tree.ready())
            .map(|t| {
                crate::app::search_tree(t, &input.text)
                    .into_iter()
                    .map(|i| t[i].path.clone())
                    .collect()
            })
            .unwrap_or_default();
        let row_h = self.f(26.0);
        let visible = results.len().min(12);
        let list_h = visible.max(1) as f32 * row_h;
        let pw2 = self.f(680.0).min(w - self.f(40.0));
        let ph = list_h + self.f(118.0);
        // Anchored near the top like a command palette.
        let r = RectF::new((w - pw2) / 2.0, h * 0.14 + lift, pw2, ph);
        self.overlay_panel(dl, atlas, r, &title);
        let count = format!("{} MATCHES", results.len());
        let cw = dl.text_width(atlas, UI, self.f(11.0), &count, self.f(1.5));
        dl.text(atlas, UI, self.f(11.0), r.right() - cw - self.f(24.0), r.y + self.f(34.0), &count, FAINT, self.f(1.5));
        let field = RectF::new(r.x + self.f(24.0), r.y + self.f(48.0), r.w - self.f(48.0), self.f(38.0));
        self.input_field(dl, atlas, &input, field, true);

        let sel = sel.min(results.len().saturating_sub(1));
        let first = if sel >= visible && visible > 0 { sel + 1 - visible } else { 0 };
        let y0 = field.bottom() + self.f(12.0);
        for vis in 0..visible {
            let i = first + vis;
            let y = y0 + vis as f32 * row_h;
            let rr = RectF::new(r.x + self.f(16.0), y, r.w - self.f(32.0), row_h - 2.0);
            let hv = self.hover_amt(wid(Z_FILE, i), rr.contains(self.mouse.0, self.mouse.1));
            let a = if i == sel { 0.13 } else { 0.06 * hv };
            if a > 0.005 {
                dl.rrect(rr, self.f(3.0), with_a(CYAN, a), 1.0);
            }
            let baseline = y + self.f(18.0);
            let path = dl.fit(atlas, MONO, self.f(12.0), &results[i], rr.w - self.f(20.0));
            // Directory part dim, filename bright.
            let (dir, name) = match path.rsplit_once('/') {
                Some((d, n)) => (format!("{}/", d), n.to_string()),
                None => (String::new(), path.clone()),
            };
            let mut x = rr.x + self.f(10.0);
            if !dir.is_empty() {
                x = dl.text(atlas, MONO, self.f(12.0), x, baseline, &dir, FAINT, 0.0);
            }
            let nc = if i == sel { CYAN } else { with_a(TEXT, 0.9) };
            dl.text(atlas, MONO, self.f(12.0), x, baseline, &name, nc, 0.0);
            self.clicks.push((rr, Click::OverlayItem(i)));
        }
        if results.is_empty() {
            dl.text(atlas, UI, self.f(13.0), r.x + self.f(24.0), y0 + self.f(16.0), "no matching files", FAINT, self.f(1.0));
        }
    }
}
