//! Drawing the wrapped transcript: code strips, selection band, label
//! and syntax-run text.

use super::agent_text::TLine;
use super::*;

impl View {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_transcript(&mut self, dl: &mut DrawList, atlas: &mut Atlas, lines: &[TLine], sel: Option<((usize, usize), (usize, usize))>, inner: RectF, row: f32, asc: f32, adv: f32, px: f32, offset: f32) {
        dl.push_clip(inner);
        let first = (offset / row) as usize;
        for vis in 0..(inner.h / row) as usize + 2 {
            let i = first + vis;
            let Some(l) = lines.get(i) else { break };
            let ytop = inner.y + i as f32 * row - offset;
            let y = ytop + asc;
            if l.code {
                dl.solid(RectF::new(inner.x - self.f(6.0), ytop, inner.w + self.f(12.0), row), BG2);
                dl.solid(RectF::new(inner.x - self.f(6.0), ytop, self.f(2.0), row), with_a(CYAN, 0.45));
            }
            // Selection band (under the text, over the code strip).
            if let Some((a, b)) = sel {
                if i >= a.0 && i <= b.0 && !(a == b) {
                    let len = l.text.chars().count();
                    let c0 = if i == a.0 { a.1.min(len) } else { 0 };
                    let c1 = if i == b.0 { b.1.min(len) } else { len + 1 };
                    if c1 > c0 {
                        let x0 = self.agent_col_x(i, c0, adv);
                        let x1 = self.agent_col_x(i, c1, adv);
                        dl.solid(RectF::new(inner.x + x0, ytop, x1 - x0, row), with_a(CYAN, 0.2));
                    }
                }
            }
            if l.text.is_empty() {
                continue;
            }
            if l.label {
                dl.text(atlas, UI_BOLD, self.f(11.5), inner.x, y, &l.text, l.color, self.f(2.5));
            } else if !l.spans.is_empty() {
                // Syntax-colored runs (fence content; tabs pre-expanded).
                let mut span_i = 0;
                let mut run = String::new();
                let mut run_color = l.color;
                let mut run_start = 0usize;
                for (ci, ch) in l.text.chars().enumerate() {
                    while span_i < l.spans.len() && l.spans[span_i].1 <= ci {
                        span_i += 1;
                    }
                    let color = l.spans
                        .get(span_i)
                        .filter(|(s, e, _)| *s <= ci && ci < *e)
                        .map(|(_, _, c)| crate::px::theme::c(*c, 1.0))
                        .unwrap_or(l.color);
                    if color != run_color && !run.is_empty() {
                        dl.text(atlas, MONO, px, inner.x + self.agent_col_x(i, run_start, adv), y, &run, run_color, 0.0);
                        run.clear();
                    }
                    if run.is_empty() {
                        run_start = ci;
                        run_color = color;
                    }
                    run.push(ch);
                }
                if !run.is_empty() {
                    dl.text(atlas, MONO, px, inner.x + self.agent_col_x(i, run_start, adv), y, &run, run_color, 0.0);
                }
            } else if l.links.is_empty() {
                dl.text(atlas, MONO, px, inner.x, y, &l.text, l.color, 0.0);
            } else {
                self.draw_links(dl, atlas, l, i, inner, ytop, y, row, adv, px);
            }
        }
        dl.pop_clip();
    }

    /// A prose line carrying hyperlinks: normal runs in the line color, link
    /// runs in cyan + underline, and a clickable hit-region per link clamped
    /// to the visible pane.
    #[allow(clippy::too_many_arguments)]
    fn draw_links(&mut self, dl: &mut DrawList, atlas: &mut Atlas, l: &TLine, i: usize, inner: RectF, ytop: f32, y: f32, row: f32, adv: f32, px: f32) {
        let chars: Vec<char> = l.text.chars().collect();
        let mut at = 0usize;
        let draw_run = |dl: &mut DrawList, atlas: &mut Atlas, s: &str, x: f32, c: Color| {
            dl.text(atlas, MONO, px, x, y, s, c, 0.0);
        };
        for &(a, b, u) in &l.links {
            let (a, b) = (a.min(chars.len()), b.min(chars.len()));
            if a < at || b <= a {
                continue; // overlapping or empty — skip defensively
            }
            if at < a {
                let run: String = chars[at..a].iter().collect();
                draw_run(dl, atlas, &run, inner.x + self.agent_col_x(i, at, adv), l.color);
            }
            let x0 = inner.x + self.agent_col_x(i, a, adv);
            let x1 = inner.x + self.agent_col_x(i, b, adv);
            let run: String = chars[a..b].iter().collect();
            draw_run(dl, atlas, &run, x0, CYAN);
            dl.solid(RectF::new(x0, y + self.f(2.0), x1 - x0, self.f(1.0)), with_a(CYAN, 0.6));
            let hit = RectF::new(x0, ytop, x1 - x0, row).intersect(&inner);
            if hit.w > 0.0 && hit.h > 0.0 {
                self.clicks.push((hit, Click::OpenUrl(u)));
            }
            at = b;
        }
        if at < chars.len() {
            let run: String = chars[at..].iter().collect();
            draw_run(dl, atlas, &run, inner.x + self.agent_col_x(i, at, adv), l.color);
        }
    }
}
