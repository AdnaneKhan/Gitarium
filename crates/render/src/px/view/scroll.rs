//! Mouse-wheel scrolling: resolve the hovered scroll target from the
//! last frame's regions and advance its smoothed offset, writing the row
//! back into App state where keyboard navigation needs it.

use super::*;

impl View {
    pub fn wheel(&mut self, app: &mut App, x: f32, y: f32, dy_px: f32) {
        let hit = self.wheels.iter().rev().find(|(r, ..)| r.contains(x, y)).map(|(_, t, rh, m)| (*t, *rh, *m));
        let Some((target, row_h, max_px)) = hit else { return };
        let s = self.scrolls.entry(skey(target)).or_insert_with(|| Smooth::new(0.0));
        // Clamp to the content extent so the wheel can't rubber-band into a
        // dead zone past either end of the list.
        s.target = (s.target + dy_px).clamp(0.0, max_px);
        // Row write-back keeps keyboard navigation coherent in App state.
        let rows = (s.target.max(0.0) / row_h) as usize;
        match target {
            Scroll::Content => {
                if let Some(rv) = &mut app.rv {
                    if let Some(f) = &mut rv.file {
                        f.editor.scroll = rows.min(f.editor.line_count().saturating_sub(1));
                        self.last_editor_scroll = f.editor.scroll;
                    }
                }
            }
            Scroll::Repos => app.repo_scroll = rows,
            Scroll::Tree => {
                if let Some(rv) = &mut app.rv {
                    rv.tree_scroll = rows;
                }
            }
            Scroll::Runs => {
                if let Some(rv) = &mut app.rv {
                    rv.runs_scroll = rows;
                }
            }
            Scroll::Jobs => {
                if let Some(rv) = &mut app.rv {
                    rv.jobs_scroll = rows;
                }
            }
            Scroll::Overlay => {
                if let Some(Overlay::BranchPick { scroll, .. }) = &mut app.overlay {
                    *scroll = rows;
                }
            }
            // Scroll state for the agent transcript lives in the Smooth only;
            // there is no keyboard row cursor to keep coherent.
            Scroll::Agent => {}
        }
        self.needs_frame = true;
    }
}
