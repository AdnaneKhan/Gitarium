//! Shared widgets: hover/scroll smoothing, panels, inputs, chips,
//! scrollbars, and the overlay panel chrome.

use super::*;

impl View {
    pub(super) fn hover_amt(&mut self, id: u64, inside: bool) -> f32 {
        let s = self.hover.entry(id).or_insert_with(|| Smooth::new(0.0));
        s.target = if inside { 1.0 } else { 0.0 };
        if s.tick_n(self.dt, 16.0) {
            self.active = true;
        }
        s.v
    }

    pub(super) fn sel_changed(&mut self, zone: u8, sel: usize) -> bool {
        let prev = self.last_sel.insert(zone, sel);
        prev != Some(sel)
    }

    /// Smooth scroll for a row list; keeps the selection visible when it
    /// moves. Returns the pixel offset.
    pub(super) fn list_scroll(&mut self, target: Scroll, zone: u8, sel: usize, count: usize, row_h: f32, view_h: f32) -> f32 {
        let changed = self.sel_changed(zone, sel);
        let s = self.scrolls.entry(skey(target)).or_insert_with(|| Smooth::new(0.0));
        let max = (count as f32 * row_h - view_h).max(0.0);
        if changed {
            let sy = sel as f32 * row_h;
            if sy < s.target {
                s.target = sy;
            }
            if sy + row_h > s.target + view_h {
                s.target = sy + row_h - view_h;
            }
        }
        s.target = s.target.clamp(0.0, max);
        if s.tick(self.dt, 14.0) {
            self.active = true;
        }
        s.v.clamp(0.0, max)
    }

    pub(super) fn brackets(&self, dl: &mut DrawList, r: RectF, len: f32, color: Color) {
        let t = self.f(2.0);
        for (cx, cy, dx, dy) in [
            (r.x, r.y, 1.0, 1.0),
            (r.right(), r.y, -1.0, 1.0),
            (r.x, r.bottom(), 1.0, -1.0),
            (r.right(), r.bottom(), -1.0, -1.0),
        ] {
            let x0 = if dx > 0.0 { cx } else { cx - len };
            let y0 = if dy > 0.0 { cy } else { cy - t };
            dl.solid(RectF::new(x0, y0, len, t), color);
            let x1 = if dx > 0.0 { cx } else { cx - t };
            let y1 = if dy > 0.0 { cy } else { cy - len };
            dl.solid(RectF::new(x1, y1, t, len), color);
        }
    }

    pub(super) fn panel(&self, dl: &mut DrawList, r: RectF) {
        dl.rrect(r, self.f(4.0), BG1, 1.0);
        dl.border(r, self.f(4.0), 1.0, BORDER_BRIGHT);
        self.brackets(dl, r, self.f(12.0), with_a(CYAN, 0.55));
    }

    pub(super) fn input_field(&mut self, dl: &mut DrawList, atlas: &mut Atlas, input: &LineInput, r: RectF, focus: bool) {
        dl.rrect(r, self.f(3.0), BG2, 1.0);
        let line = RectF::new(r.x, r.bottom() - self.f(2.0), r.w, self.f(2.0));
        if focus {
            dl.glow(line, 1.0, with_a(CYAN, 0.3), self.f(7.0));
            dl.solid(line, with_a(CYAN, 0.9));
        } else {
            dl.solid(line, BORDER_BRIGHT);
        }
        let px = self.f(14.0);
        let (ascent, lh) = atlas.metrics(MONO, px);
        let adv = atlas.advance(MONO, px, 'M');
        let pad = self.f(12.0);
        let shown: String = if input.masked {
            "•".repeat(input.text.chars().count())
        } else {
            input.text.clone()
        };
        let visible = (((r.w - pad * 2.0) / adv) as usize).max(4);
        let cur = input.cursor;
        let off = (cur + 1).saturating_sub(visible);
        let slice: String = shown.chars().skip(off).take(visible).collect();
        let baseline = r.y + (r.h - lh) / 2.0 + ascent;
        dl.text(atlas, MONO, px, r.x + pad, baseline, &slice, TEXT, 0.0);
        if focus {
            self.active = true; // caret blink
            if ((self.time / 530.0) as i64) % 2 == 0 {
                let cx = r.x + pad + (cur - off) as f32 * adv;
                let cr = RectF::new(cx, baseline - ascent, self.f(2.0), lh);
                dl.glow(cr, 1.0, with_a(CYAN, 0.4), self.f(4.0));
                dl.solid(cr, CYAN);
            }
        }
    }

    /// Right-aligned action chip; returns the new right edge for stacking.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn chip(&mut self, dl: &mut DrawList, atlas: &mut Atlas, label: &str, right: f32, cy: f32, color: Color, click: Click, id: u64) -> f32 {
        let px = self.f(12.0);
        let tw = dl.text_width(atlas, UI_BOLD, px, label, self.f(1.5));
        let r = RectF::new(right - tw - self.f(20.0), cy - self.f(11.0), tw + self.f(20.0), self.f(22.0));
        let hv = self.hover_amt(id, r.contains(self.hot.0, self.hot.1));
        if hv > 0.01 {
            dl.glow(r, self.f(3.0), with_a(color, 0.20 * hv), self.f(9.0));
        }
        dl.rrect(r, self.f(3.0), with_a(color, 0.07 + 0.10 * hv), 1.0);
        dl.border(r, self.f(3.0), 1.0, with_a(color, 0.75));
        let (ascent, lh) = atlas.metrics(UI_BOLD, px);
        dl.text(atlas, UI_BOLD, px, r.x + self.f(10.0), r.y + (r.h - lh) / 2.0 + ascent, label, color, self.f(1.5));
        self.clicks.push((r, click));
        r.x - self.f(8.0)
    }

    /// Left-aligned toolbar chip; returns the next x.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn tool_chip(&mut self, dl: &mut DrawList, atlas: &mut Atlas, label: &str, x: f32, y: f32, color: Color, click: Click, id: u64) -> f32 {
        let px = self.f(11.0);
        let tw = dl.text_width(atlas, UI_BOLD, px, label, self.f(1.2));
        let r = RectF::new(x, y, tw + self.f(18.0), self.f(24.0));
        let hv = self.hover_amt(id, r.contains(self.hot.0, self.hot.1));
        if hv > 0.01 {
            dl.glow(r, self.f(3.0), with_a(color, 0.18 * hv), self.f(8.0));
        }
        dl.rrect(r, self.f(3.0), with_a(color, 0.05 + 0.08 * hv), 1.0);
        dl.border(r, self.f(3.0), 1.0, with_a(color, 0.55 + 0.35 * hv));
        let (asc, lh) = atlas.metrics(UI_BOLD, px);
        dl.text(atlas, UI_BOLD, px, r.x + self.f(9.0), r.y + (r.h - lh) / 2.0 + asc, label, color, self.f(1.2));
        self.clicks.push((r, click));
        r.right() + self.f(8.0)
    }

    pub(super) fn sweep_note(&mut self, dl: &mut DrawList, atlas: &mut Atlas, x: f32, y: f32, w: f32, label: &str) {
        let px = self.f(13.0);
        dl.text(atlas, UI, px, x, y, label, DIM, self.f(2.0));
        let bar = RectF::new(x, y + self.f(10.0), w.min(self.f(220.0)), self.f(2.0));
        let p = ((self.time * 0.0009) % 1.0) as f32;
        dl.solid(bar, with_a(CYAN, 0.10));
        let pw = bar.w * 0.3;
        dl.push_clip(bar);
        dl.solid(RectF::new(bar.x + p * bar.w - pw, bar.y, pw, bar.h), with_a(CYAN, 0.8));
        dl.pop_clip();
        self.active = true;
    }

    pub(super) fn scrollbar(&self, dl: &mut DrawList, area: &RectF, content_h: f32, offset: f32) {
        if content_h <= area.h {
            return;
        }
        let track = RectF::new(area.right() - self.f(4.0), area.y, self.f(3.0), area.h);
        dl.rrect(track, self.f(1.5), with_a(CYAN, 0.06), 1.0);
        let th = (area.h / content_h * area.h).max(self.f(24.0));
        let max = content_h - area.h;
        let ty = area.y + (offset / max) * (area.h - th);
        dl.rrect(RectF::new(track.x, ty, track.w, th), self.f(1.5), with_a(CYAN, 0.45), 1.0);
    }

    pub(super) fn overlay_panel(&mut self, dl: &mut DrawList, atlas: &mut Atlas, r: RectF, title: &str) {
        dl.glow(r, self.f(4.0), with_a(CYAN, 0.07), self.f(30.0));
        dl.rrect(r, self.f(4.0), BG1, 1.0);
        dl.border(r, self.f(4.0), 1.0, BORDER_BRIGHT);
        self.brackets(dl, r, self.f(12.0), with_a(CYAN, 0.7));
        dl.text(atlas, UI_BOLD, self.f(16.0), r.x + self.f(24.0), r.y + self.f(34.0), title, CYAN, self.f(3.0));
    }
}
