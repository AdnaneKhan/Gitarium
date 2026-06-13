//! Glyph-run tinting for the Canvas2D backend. The atlas holds coverage as
//! white-with-alpha; a run of same-color glyphs is blitted onto a scratch
//! canvas, tinted in one source-in fill, and composited back — four canvas
//! ops per text run instead of per glyph.

use super::canvas2d::{css, State};

pub(super) struct GlyphRun {
    pub(super) color: [f32; 4],
    /// (atlas src x/y/w/h, dest x/y/w/h) per glyph, in canvas pixels.
    items: Vec<([f32; 4], [f32; 4])>,
    min: (f32, f32),
    max: (f32, f32),
}

impl GlyphRun {
    pub(super) fn new(color: [f32; 4]) -> Self {
        GlyphRun { color, items: Vec::new(), min: (f32::MAX, f32::MAX), max: (f32::MIN, f32::MIN) }
    }

    pub(super) fn push(&mut self, src: [f32; 4], dst: [f32; 4]) {
        self.min = (self.min.0.min(dst[0]), self.min.1.min(dst[1]));
        self.max = (self.max.0.max(dst[0] + dst[2]), self.max.1.max(dst[1] + dst[3]));
        self.items.push((src, dst));
    }

    pub(super) fn draw(self, s: &State) {
        if self.items.is_empty() {
            return;
        }
        // Pad by a pixel so fractional positions don't clip at the bbox.
        let (ox, oy) = (self.min.0.floor() - 1.0, self.min.1.floor() - 1.0);
        let bw = (self.max.0 - ox).ceil() as u32 + 1;
        let bh = (self.max.1 - oy).ceil() as u32 + 1;
        if s.scratch.width() < bw {
            s.scratch.set_width(bw.next_power_of_two());
        }
        if s.scratch.height() < bh {
            s.scratch.set_height(bh.next_power_of_two());
        }
        let sc = &s.scratch_ctx;
        let _ = sc.set_global_composite_operation("source-over");
        sc.clear_rect(0.0, 0.0, s.scratch.width() as f64, s.scratch.height() as f64);
        for (src, dst) in &self.items {
            let _ = sc.draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &s.atlas_canvas,
                src[0] as f64,
                src[1] as f64,
                src[2] as f64,
                src[3] as f64,
                (dst[0] - ox) as f64,
                (dst[1] - oy) as f64,
                dst[2] as f64,
                dst[3] as f64,
            );
        }
        // Tint: keep coverage alpha, replace color (alpha multiplies).
        let _ = sc.set_global_composite_operation("source-in");
        sc.set_fill_style_str(&css(self.color));
        sc.fill_rect(0.0, 0.0, bw as f64, bh as f64);
        let _ = s.ctx.draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            &s.scratch,
            0.0,
            0.0,
            bw as f64,
            bh as f64,
            ox as f64,
            oy as f64,
            bw as f64,
            bh as f64,
        );
    }
}
