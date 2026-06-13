//! Painting laid-out markdown rows: prose with inline emphasis / code pills /
//! links, fenced-code strips, tables, and thematic breaks. Link spans become
//! `Click::OpenUrl` hit-regions (indices offset into the frame's url table).

use super::layout::measure;
use super::{select, *};

impl View {
    /// Draw the visible window of `rows` within `area`, scrolled by `offset`
    /// px, at uniform `row_h`. `urls` is this block's link table; its entries
    /// are appended to the frame url table so clicks resolve. Records each
    /// row's text + geometry into the shared selection state so the body is
    /// mouse-selectable and copyable (reusing the transcript machinery).
    #[allow(clippy::too_many_arguments)]
    pub(in crate::px::view) fn draw_markdown(&mut self, dl: &mut DrawList, atlas: &mut Atlas, rows: &[MdRow], urls: &[String], area: RectF, row_h: f32, offset: f32) {
        let base_url = self.link_urls.len();
        self.link_urls.extend(urls.iter().cloned());

        // Selection model. A live selection that no longer lines up with the
        // current rows (content loaded/changed) is dropped, not re-targeted.
        let texts: Vec<String> = rows.iter().map(select::row_text).collect();
        if let Some((a, b)) = self.agent_sel {
            let hi = a.0.max(b.0);
            let intact = hi < texts.len()
                && hi < self.agent_lines.len()
                && (0..=hi).all(|i| texts[i] == self.agent_lines[i]);
            if !intact {
                self.agent_sel = None;
            }
        }
        let code_x = self.f(12.0);
        self.agent_xs = rows.iter().map(|r| select::row_xs(atlas, r, code_x)).collect();
        self.agent_src = (0..rows.len()).map(|i| Some(i as u32)).collect();
        self.agent_lines = texts;
        let adv = atlas.advance(MONO, self.f(12.5), 'M');
        self.agent_geom = Some((area, row_h, adv, offset));
        if self.drag == Drag::Agent && self.agent_sel.is_some() {
            if let Some(pos) = self.agent_pos_at(self.mouse.0, self.mouse.1, true) {
                if let Some(sel) = &mut self.agent_sel {
                    sel.1 = pos;
                }
            }
        }
        let sel = self.agent_sel.map(|(a, b)| if a <= b { (a, b) } else { (b, a) });

        dl.push_clip(area);
        let first = (offset / row_h) as usize;
        for vis in 0..(area.h / row_h) as usize + 2 {
            let i = first + vis;
            let Some(row) = rows.get(i) else { break };
            let ytop = area.y + i as f32 * row_h - offset;
            if let Some((a, b)) = sel {
                if i >= a.0 && i <= b.0 && a != b && !matches!(row, MdRow::Table { .. }) {
                    let len = self.agent_lines[i].chars().count();
                    let c0 = if i == a.0 { a.1.min(len) } else { 0 };
                    let c1 = if i == b.0 { b.1.min(len) } else { len + 1 };
                    if c1 > c0 {
                        let x0 = area.x + self.agent_col_x(i, c0, adv);
                        let x1 = area.x + self.agent_col_x(i, c1, adv);
                        dl.solid(RectF::new(x0, ytop, x1 - x0, row_h), with_a(CYAN, 0.2));
                    }
                }
            }
            self.md_row(dl, atlas, row, area, ytop, row_h, base_url);
        }
        dl.pop_clip();
    }

    #[allow(clippy::too_many_arguments)]
    fn md_row(&mut self, dl: &mut DrawList, atlas: &mut Atlas, row: &MdRow, area: RectF, ytop: f32, row_h: f32, base_url: usize) {
        match row {
            MdRow::Blank => {}
            MdRow::Rule => {
                let y = ytop + (row_h * 0.5).round();
                dl.solid(RectF::new(area.x + self.f(6.0), y, area.w - self.f(12.0), 1.0), BORDER_BRIGHT);
            }
            MdRow::Code { spans, .. } => {
                dl.solid(RectF::new(area.x, ytop, area.w, row_h), BG2);
                dl.solid(RectF::new(area.x, ytop, self.f(2.0), row_h), with_a(CYAN, 0.45));
                let px = spans.first().map(|s| s.px).unwrap_or(self.f(12.0));
                let (asc, lh) = atlas.metrics(MONO, px);
                let y = ytop + asc + (row_h - lh) * 0.5;
                let mut x = area.x + self.f(12.0);
                for s in spans {
                    x = dl.text(atlas, MONO, s.px, x, y, &s.text, s.color, 0.0);
                }
            }
            MdRow::Table { cells, widths, aligns, header } => {
                self.md_table(dl, atlas, cells, widths, aligns, *header, area, ytop, row_h, base_url);
            }
            MdRow::Line { spans, indent, deco } => {
                self.md_line(dl, atlas, spans, *indent, deco, area, ytop, row_h, base_url);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn md_line(&mut self, dl: &mut DrawList, atlas: &mut Atlas, spans: &[Span], indent: f32, deco: &Deco, area: RectF, ytop: f32, row_h: f32, base_url: usize) {
        let x0 = area.x + indent;
        let (font, px) = spans.first().map(|s| (s.font, s.px)).unwrap_or((UI, self.f(13.0)));
        let (asc, lh) = atlas.metrics(font, px);
        let y = ytop + asc + (row_h - lh) * 0.5;
        // Left-edge decoration.
        match deco {
            Deco::Quote(d) => {
                for k in 0..*d {
                    let bx = area.x + k as f32 * self.f(10.0) + self.f(2.0);
                    dl.solid(RectF::new(bx, ytop, self.f(2.0), row_h), with_a(CYAN, 0.4));
                }
            }
            Deco::Marker(m) => {
                let mw = measure(atlas, UI, px, m);
                dl.text(atlas, UI, px, x0 - mw - self.f(5.0), y, m, with_a(CYAN, 0.85), 0.0);
            }
            Deco::Task(done) => {
                let bs = self.f(11.0);
                let r = RectF::new(x0 - bs - self.f(6.0), ytop + (row_h - bs) * 0.5, bs, bs);
                dl.border(r, self.f(2.0), 1.0, with_a(CYAN, 0.7));
                if *done {
                    dl.text(atlas, UI_BOLD, px, r.x - self.f(0.5), y, "✓", GREEN, 0.0);
                }
            }
            _ => {}
        }
        // Pre-measure span extents for code pills, underlines, and link hits.
        let mut xs = Vec::with_capacity(spans.len() + 1);
        let mut x = x0;
        for s in spans {
            xs.push(x);
            x += measure(atlas, s.font, s.px, &s.text);
        }
        xs.push(x);
        for (k, s) in spans.iter().enumerate() {
            if s.code {
                let r = RectF::new(xs[k] - self.f(2.0), ytop + self.f(2.0), xs[k + 1] - xs[k] + self.f(4.0), row_h - self.f(4.0));
                dl.rrect(r, self.f(3.0), with_a(CYAN, 0.09), 1.0);
            }
        }
        for (k, s) in spans.iter().enumerate() {
            dl.text(atlas, s.font, s.px, xs[k], y, &s.text, s.color, 0.0);
            if s.strike {
                dl.solid(RectF::new(xs[k], y - lh * 0.28, xs[k + 1] - xs[k], self.f(1.0)), s.color);
            }
            if let Some(u) = s.link {
                dl.solid(RectF::new(xs[k], y + self.f(2.0), xs[k + 1] - xs[k], self.f(1.0)), with_a(CYAN, 0.6));
                self.link_hit(xs[k], ytop, xs[k + 1] - xs[k], row_h, area, base_url + u);
            }
        }
        if let Deco::Heading(l) = deco {
            if *l <= 2 {
                dl.solid(RectF::new(x0, ytop + row_h - self.f(3.0), (area.right() - x0 - self.f(6.0)).max(0.0), 1.0), with_a(CYAN, 0.3));
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn md_table(&mut self, dl: &mut DrawList, atlas: &mut Atlas, cells: &[Vec<Span>], widths: &[f32], aligns: &[Align], header: bool, area: RectF, ytop: f32, row_h: f32, base_url: usize) {
        let px = cells.iter().flatten().next().map(|s| s.px).unwrap_or(self.f(13.0));
        let (asc, lh) = atlas.metrics(if header { UI_BOLD } else { UI }, px);
        let y = ytop + asc + (row_h - lh) * 0.5;
        if header {
            dl.solid(RectF::new(area.x, ytop, area.w, row_h), with_a(CYAN, 0.06));
        }
        let pad = self.f(6.0);
        let mut cx = area.x + self.f(2.0);
        for (c, cell) in cells.iter().enumerate() {
            let cw = widths.get(c).copied().unwrap_or(self.f(80.0));
            let content: f32 = cell.iter().map(|s| measure(atlas, s.font, s.px, &s.text)).sum();
            let avail = (cw - pad * 2.0).max(0.0);
            let startx = match aligns.get(c).copied().unwrap_or(Align::Left) {
                Align::Left => cx + pad,
                Align::Right => cx + cw - pad - content.min(avail),
                Align::Center => cx + pad + (avail - content.min(avail)) * 0.5,
            };
            dl.push_clip(RectF::new(cx, ytop, cw, row_h));
            let mut x = startx;
            for s in cell {
                let nx = dl.text(atlas, s.font, s.px, x, y, &s.text, s.color, 0.0);
                if let Some(u) = s.link {
                    self.link_hit(x, ytop, nx - x, row_h, area, base_url + u);
                }
                x = nx;
            }
            dl.pop_clip();
            dl.solid(RectF::new(cx + cw, ytop, 1.0, row_h), with_a(BORDER, 0.7));
            cx += cw;
        }
        let line = if header { BORDER_BRIGHT } else { with_a(BORDER, 0.5) };
        dl.solid(RectF::new(area.x, ytop + row_h - 1.0, area.w, 1.0), line);
    }

    fn link_hit(&mut self, x: f32, ytop: f32, w: f32, h: f32, area: RectF, url: usize) {
        let hit = RectF::new(x, ytop, w, h).intersect(&area);
        if hit.w > 0.0 && hit.h > 0.0 {
            self.clicks.push((hit, Click::OpenUrl(url)));
        }
    }
}
