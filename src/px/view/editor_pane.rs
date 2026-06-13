//! The editor/content body: gutter, syntax runs, selection, caret.

use super::*;

impl View {
    pub(super) fn editor_body(&mut self, app: &mut App, dl: &mut DrawList, atlas: &mut Atlas, body: RectF) {
        let code_px = self.f(13.5);
        let (ascent, lh) = atlas.metrics(MONO, code_px);
        let line_h = (lh * 1.08).ceil();
        let adv = atlas.advance(MONO, code_px, 'M');

        // Sync keyboard-driven row scrolling into the smooth pixel offset.
        {
            let rv = app.rv.as_mut().unwrap();
            let f = rv.file.as_mut().unwrap();
            if f.editor.scroll != self.last_editor_scroll {
                self.last_editor_scroll = f.editor.scroll;
                let s = self.scrolls.entry(skey(Scroll::Content)).or_insert_with(|| Smooth::new(0.0));
                s.target = f.editor.scroll as f32 * line_h;
            }
        }

        let rv = app.rv.as_ref().unwrap();
        let file = rv.file.as_ref().unwrap();
        let ed = &file.editor;
        let total = ed.line_count();
        let digits = total.to_string().len().max(3);
        let gutter = digits as f32 * adv + self.f(20.0);
        let text_rect = RectF::new(body.x + gutter, body.y, body.w - gutter - self.f(10.0), body.h);
        let editing = file.editing && rv.focus == RepoFocus::Content;

        let s = self.scrolls.entry(skey(Scroll::Content)).or_insert_with(|| Smooth::new(0.0));
        let max = (total as f32 * line_h - body.h).max(0.0);
        s.target = s.target.clamp(0.0, max);
        if s.tick(self.dt, 14.0) {
            self.active = true;
        }
        let offset = s.v.clamp(0.0, max);
        let hscroll_px = ed.hscroll as f32 * adv;

        dl.push_clip(body);
        let first = (offset / line_h) as usize;
        let vis_rows = (body.h / line_h) as usize + 2;
        let sel = ed.sel_range();
        let caret_on = ((self.time / 530.0) as i64) % 2 == 0;
        for vis in 0..vis_rows {
            let row = first + vis;
            if row >= total {
                break;
            }
            let y = body.y + row as f32 * line_h - offset;
            let baseline = y + ascent;
            let cursor_row = row == ed.cursor.0;

            // Gutter.
            let num = format!("{:>w$}", row + 1, w = digits);
            let nc = if cursor_row && editing { with_a(CYAN, 0.9) } else { FAINT };
            dl.text(atlas, MONO, self.f(11.0), body.x, baseline, &num, nc, 0.0);

            let line = &ed.lines[row];
            // Selection band.
            if let Some((a, b)) = sel {
                if row >= a.0 && row <= b.0 {
                    let x0 = if row == a.0 { ed.col_to_x(row, a.1) as f32 * adv } else { 0.0 };
                    let x1 = if row == b.0 {
                        ed.col_to_x(row, b.1) as f32 * adv
                    } else {
                        (ed.col_to_x(row, line.chars().count()) + 1) as f32 * adv
                    };
                    dl.solid(
                        RectF::new(text_rect.x + x0 - hscroll_px, y, (x1 - x0).max(adv * 0.4), line_h),
                        with_a(CYAN, 0.15),
                    );
                }
            }

            // Syntax-colored runs.
            let state = file.line_states.get(row).copied().unwrap_or(LineState::Normal);
            let spans = match file.lang {
                Some(spec) => highlight::highlight(spec, line, state).0,
                None => Vec::new(),
            };
            let mut span_i = 0;
            let mut run = String::new();
            let mut run_color = TEXT;
            let mut run_start_cell = 0usize;
            let mut cell = 0usize;
            let flush = |dl: &mut DrawList, atlas: &mut Atlas, run: &mut String, start: usize, color: Color| {
                if !run.is_empty() {
                    let x = text_rect.x + start as f32 * adv - hscroll_px;
                    dl.text(atlas, MONO, code_px, x, baseline, run, color, 0.0);
                    run.clear();
                }
            };
            for (ci, ch) in line.chars().enumerate() {
                while span_i < spans.len() && spans[span_i].1 <= ci {
                    span_i += 1;
                }
                let color = spans
                    .get(span_i)
                    .filter(|(st, en, _)| *st <= ci && ci < *en)
                    .map(|(_, _, c)| crate::px::theme::c(*c, 1.0))
                    .unwrap_or(TEXT);
                if ch == '\t' {
                    flush(dl, atlas, &mut run, run_start_cell, run_color);
                    cell += crate::app::editor::TAB_W;
                    run_start_cell = cell;
                    continue;
                }
                if color != run_color && !run.is_empty() {
                    flush(dl, atlas, &mut run, run_start_cell, run_color);
                    run_start_cell = cell;
                }
                if run.is_empty() {
                    run_start_cell = cell;
                    run_color = color;
                }
                run.push(ch);
                cell += 1;
            }
            flush(dl, atlas, &mut run, run_start_cell, run_color);

            // Caret.
            if editing && cursor_row {
                self.active = true;
                if caret_on {
                    let cx = text_rect.x + ed.col_to_x(row, ed.cursor.1) as f32 * adv - hscroll_px;
                    let cr = RectF::new(cx, y + 1.0, self.f(2.0), line_h - 2.0);
                    dl.glow(cr, 1.0, with_a(CYAN, 0.45), self.f(5.0));
                    dl.solid(cr, CYAN);
                }
            }
        }
        dl.pop_clip();
        self.scrollbar(dl, &body, total as f32 * line_h, offset);
        self.wheels.push((body, Scroll::Content, line_h, max));
        self.editor_geom = Some(EditorGeom {
            rect: text_rect,
            line_h,
            adv,
            scroll_px: offset,
            hscroll_px,
        });
        app.layout.content_text = CellRect::new(
            0,
            0,
            ((text_rect.w / adv) as i32).max(10),
            ((body.h / line_h) as i32).max(3),
        );
        app.layout.gutter = gutter as i32;
    }
}
