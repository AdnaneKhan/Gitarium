//! The file-tree pane of the Code tab.

use super::*;

impl View {
    pub(super) fn tree_pane(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, tree: RectF, inner: RectF, row_h: f32) {
        enum TreeState {
            Loading,
            Failed(String),
            Ready,
        }
        let state = match &app.rv.as_ref().unwrap().tree {
            Loadable::Loading | Loadable::Idle => TreeState::Loading,
            Loadable::Failed(e) => TreeState::Failed(e.clone()),
            Loadable::Ready(_) => TreeState::Ready,
        };
        match state {
            TreeState::Loading => {
                self.sweep_note(dl, atlas, inner.x + self.f(6.0), inner.y + self.f(22.0), inner.w, "SCANNING TREE…")
            }
            TreeState::Failed(e) => {
                let msg = dl.fit(atlas, UI, self.f(13.0), &e, inner.w - self.f(8.0));
                dl.text(atlas, UI, self.f(13.0), inner.x + self.f(6.0), inner.y + self.f(22.0), &msg, RED, 0.0);
            }
            TreeState::Ready => {
                let (sel, count, focus_tree, truncated) = {
                    let rv = app.rv.as_ref().unwrap();
                    (rv.tree_sel, rv.rows.len(), rv.focus == RepoFocus::Tree, rv.truncated)
                };
                let offset = self.list_scroll(Scroll::Tree, Z_TREE, sel, count, row_h, inner.h);
                dl.push_clip(inner);
                {
                    let rv = app.rv.as_ref().unwrap();
                    let first = (offset / row_h) as usize;
                    for vis in 0..(inner.h / row_h) as usize + 2 {
                        let i = first + vis;
                        if i >= rv.rows.len() {
                            break;
                        }
                        let row = &rv.rows[i];
                        let y = inner.y + i as f32 * row_h - offset;
                        let rr = RectF::new(inner.x, y, inner.w, row_h - 1.0);
                        let selected = i == sel;
                        let hv = self.hover_amt(wid(Z_TREE, i), rr.contains(self.hot.0, self.hot.1));
                        let a = if selected {
                            if focus_tree {
                                0.12
                            } else {
                                0.06
                            }
                        } else {
                            0.05 * hv
                        };
                        if a > 0.005 {
                            dl.rrect(rr, self.f(3.0), with_a(CYAN, a), 1.0);
                        }
                        let ix = inner.x + self.f(8.0) + row.depth as f32 * self.f(15.0);
                        let (asc, _) = atlas.metrics(MONO, self.f(12.0));
                        let baseline = y + (row_h + asc) / 2.0 - self.f(2.0);
                        let (mark, mc) = if row.is_dir {
                            if rv.expanded.contains(&row.path) {
                                ("▾", CYAN)
                            } else {
                                ("▸", CYAN)
                            }
                        } else {
                            ("·", FAINT)
                        };
                        dl.text(atlas, MONO, self.f(12.0), ix, baseline, mark, mc, 0.0);
                        let nc = if row.is_dir {
                            with_a(CYAN, 0.9)
                        } else if selected {
                            TEXT
                        } else {
                            with_a(TEXT, 0.85)
                        };
                        let name = dl.fit(atlas, UI, self.f(14.0), &row.name, inner.right() - ix - self.f(22.0));
                        dl.text(atlas, UI, self.f(14.0), ix + self.f(16.0), baseline, &name, nc, 0.0);
                        self.clicks.push((rr, Click::TreeRow(i)));
                    }
                }
                dl.pop_clip();
                self.scrollbar(dl, &inner, count as f32 * row_h, offset);
                self.wheels.push((tree, Scroll::Tree, row_h, (count as f32 * row_h - inner.h).max(0.0)));
                if truncated {
                    dl.text(atlas, UI, self.f(11.0), tree.x + self.f(10.0), tree.bottom() - self.f(8.0), "⚠ TREE TRUNCATED", YELLOW, self.f(1.0));
                }
            }
        }
    }
}
