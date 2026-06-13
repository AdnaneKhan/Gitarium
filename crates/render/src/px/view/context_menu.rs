//! The floating right-click menu: resolving a right-click against the tree,
//! and drawing the menu above everything with its item hit-regions recorded.

use super::*;

impl View {
    /// Right-click at (x, y): open the tree menu for the row under the cursor
    /// (or empty tree space), else dismiss any open menu. Uses the previous
    /// frame's tree hit-regions to resolve which row was clicked.
    pub fn on_context_menu(&mut self, app: &mut App, x: f32, y: f32) {
        self.needs_frame = true;
        let row = self
            .clicks
            .iter()
            .rev()
            .find(|(r, c)| r.contains(x, y) && matches!(c, Click::TreeRow(_)))
            .map(|(_, c)| *c);
        let hit = match row {
            Some(Click::TreeRow(i)) => {
                app.rv.as_ref().and_then(|rv| rv.rows.get(i)).map(|r| (r.path.clone(), r.is_dir))
            }
            _ => None,
        };
        let in_tree = self.tree_rect.map(|t| t.contains(x, y)).unwrap_or(false);
        if hit.is_some() || in_tree {
            app.open_tree_menu(x, y, hit);
        } else {
            app.context_menu = None;
        }
    }

    /// Draw the context menu (if open) and record per-item hit-regions. Called
    /// last in the frame so it floats above the panels and overlays.
    pub(super) fn draw_menu(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, w: f32, h: f32) {
        self.menu_rects.clear();
        let Some(menu) = &app.context_menu else { return };
        if menu.items.is_empty() {
            return;
        }
        self.active = true;
        let labels: Vec<String> = menu.items.iter().map(|i| i.label.clone()).collect();
        let fs = self.f(13.0);
        let row_h = self.f(26.0);
        let pad = self.f(6.0);
        let tw = labels
            .iter()
            .map(|l| dl.text_width(atlas, UI, fs, l, 0.0))
            .fold(0.0_f32, f32::max);
        let pw = (tw + self.f(28.0)).clamp(self.f(140.0), self.f(280.0));
        let ph = labels.len() as f32 * row_h + pad * 2.0;
        // Keep the panel on-screen near the cursor.
        let x = menu.x.min(w - pw - self.f(4.0)).max(self.f(4.0));
        let y = menu.y.min(h - ph - self.f(4.0)).max(self.f(4.0));
        let r = RectF::new(x, y, pw, ph);
        dl.glow(r, self.f(6.0), [0.0, 0.0, 0.0, 0.5], self.f(10.0));
        dl.rrect(r, self.f(6.0), [0.03, 0.05, 0.09, 0.99], 1.0);
        dl.border(r, self.f(6.0), 1.0, with_a(CYAN, 0.4));
        for (i, label) in labels.iter().enumerate() {
            let rr = RectF::new(r.x + pad, r.y + pad + i as f32 * row_h, r.w - pad * 2.0, row_h);
            let hv = self.hover_amt(wid(Z_MENU, i), rr.contains(self.mouse.0, self.mouse.1));
            if hv > 0.005 {
                dl.rrect(rr, self.f(3.0), with_a(CYAN, 0.12 * hv), 1.0);
            }
            let danger = label.starts_with("Delete");
            let col = if danger { with_a(RED, 0.95) } else { TEXT };
            let (asc, _) = atlas.metrics(UI, fs);
            dl.text(atlas, UI, fs, rr.x + self.f(10.0), rr.y + (row_h + asc) / 2.0 - self.f(2.0), label, col, 0.0);
            self.menu_rects.push((rr, i));
        }
    }
}
