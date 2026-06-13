//! Text drawing and measurement over the glyph atlas.

use super::super::atlas::Atlas;
use super::super::theme::Color;
use super::{DrawList, MODE_GLYPH};

impl DrawList {
    /// Draw a text run; returns the end pen x. `tracking` adds px between chars.
    pub fn text(
        &mut self,
        atlas: &mut Atlas,
        font: u8,
        px: f32,
        x: f32,
        baseline: f32,
        s: &str,
        c: Color,
        tracking: f32,
    ) -> f32 {
        if !s.is_empty() {
            self.dbg.push(s.to_string());
        }
        let mut pen = x;
        let mut prev: Option<char> = None;
        for ch in s.chars() {
            if let Some(p) = prev {
                pen += atlas.kern(font, px, p, ch);
            }
            if let Some(g) = atlas.glyph(font, px, ch) {
                // Snap to whole pixels: with linear filtering, fractional
                // offsets smear small glyphs.
                let gx = (pen + g.left).round();
                let gy = (baseline + g.top).round();
                self.quad(
                    gx,
                    gy,
                    gx + g.w,
                    gy + g.h,
                    g.uv,
                    c,
                    [0.0; 4],
                    [0.0, 0.0, 0.0, MODE_GLYPH],
                );
                pen += g.advance + tracking;
            } else {
                pen += atlas.advance(font, px, ch) + tracking;
            }
            prev = Some(ch);
        }
        pen
    }

    pub fn text_width(&self, atlas: &Atlas, font: u8, px: f32, s: &str, tracking: f32) -> f32 {
        let mut w = 0.0;
        let mut prev: Option<char> = None;
        for ch in s.chars() {
            if let Some(p) = prev {
                w += atlas.kern(font, px, p, ch);
            }
            w += atlas.advance(font, px, ch) + tracking;
            prev = Some(ch);
        }
        w
    }

    /// Truncate `s` with an ellipsis to fit `max_w`.
    pub fn fit(&self, atlas: &Atlas, font: u8, px: f32, s: &str, max_w: f32) -> String {
        if self.text_width(atlas, font, px, s, 0.0) <= max_w {
            return s.to_string();
        }
        let ell = atlas.advance(font, px, '…');
        let mut w = 0.0;
        let mut out = String::new();
        for ch in s.chars() {
            let a = atlas.advance(font, px, ch);
            if w + a + ell > max_w {
                break;
            }
            w += a;
            out.push(ch);
        }
        out.push('…');
        out
    }
}
