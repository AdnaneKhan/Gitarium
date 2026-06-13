//! Draw list: emits an interleaved vertex stream consumed by px::gl in one
//! draw call. Primitives: SDF rounded rects (fill / border / glow), solid
//! quads, glyph quads, scanlines. Clipping is done on the CPU since every
//! clip in this UI is an axis-aligned rect.

use super::theme::Color;

pub const MODE_GLYPH: f32 = 0.0;
pub const MODE_SDF: f32 = 1.0;
pub const MODE_SOLID: f32 = 2.0;
pub const MODE_SCAN: f32 = 3.0;
/// Color-emoji quad: sample the RGBA color atlas (straight color, not tinted).
pub const MODE_EMOJI: f32 = 4.0;

pub const FLOATS_PER_VERT: usize = 16; // pos2 uv2 color4 rect4 param4

#[derive(Clone, Copy, Debug)]
pub struct RectF {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl RectF {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        RectF { x, y, w, h }
    }
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }
    pub fn shrink(&self, n: f32) -> RectF {
        RectF::new(self.x + n, self.y + n, (self.w - 2.0 * n).max(0.0), (self.h - 2.0 * n).max(0.0))
    }
    pub fn intersect(&self, o: &RectF) -> RectF {
        let x = self.x.max(o.x);
        let y = self.y.max(o.y);
        let r = self.right().min(o.right());
        let b = self.bottom().min(o.bottom());
        RectF::new(x, y, (r - x).max(0.0), (b - y).max(0.0))
    }
}

pub struct DrawList {
    pub verts: Vec<f32>,
    clip: Vec<RectF>,
    /// Every text run drawn this frame — the browser test harness reads this.
    pub dbg: Vec<String>,
}

impl DrawList {
    pub fn new() -> Self {
        DrawList {
            verts: Vec::with_capacity(64 * 1024),
            clip: vec![RectF::new(-1e6, -1e6, 2e6, 2e6)],
            dbg: Vec::new(),
        }
    }

    pub fn begin(&mut self, w: f32, h: f32) {
        self.verts.clear();
        self.dbg.clear();
        self.clip.clear();
        self.clip.push(RectF::new(0.0, 0.0, w, h));
    }

    pub fn push_clip(&mut self, r: RectF) {
        let top = *self.clip.last().unwrap();
        self.clip.push(top.intersect(&r));
    }

    pub fn pop_clip(&mut self) {
        if self.clip.len() > 1 {
            self.clip.pop();
        }
    }

    fn vert(&mut self, x: f32, y: f32, u: f32, v: f32, c: Color, rect: [f32; 4], param: [f32; 4]) {
        self.verts.extend_from_slice(&[
            x, y, u, v, c[0], c[1], c[2], c[3], rect[0], rect[1], rect[2], rect[3], param[0],
            param[1], param[2], param[3],
        ]);
    }

    /// Clipped quad. For glyph quads, uvs are remapped to the clipped extent.
    #[allow(clippy::too_many_arguments)]
    fn quad(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, uv: [f32; 4], c: Color, rect: [f32; 4], param: [f32; 4]) {
        let clip = *self.clip.last().unwrap();
        let cx0 = x0.max(clip.x);
        let cy0 = y0.max(clip.y);
        let cx1 = x1.min(clip.right());
        let cy1 = y1.min(clip.bottom());
        if cx0 >= cx1 || cy0 >= cy1 || c[3] <= 0.003 {
            return;
        }
        let (mut u0, mut v0, mut u1, mut v1) = (uv[0], uv[1], uv[2], uv[3]);
        if (param[3] == MODE_GLYPH || param[3] == MODE_EMOJI) && (x1 - x0) > 0.0 && (y1 - y0) > 0.0 {
            let fu = (u1 - u0) / (x1 - x0);
            let fv = (v1 - v0) / (y1 - y0);
            u0 += (cx0 - x0) * fu;
            u1 -= (x1 - cx1) * fu;
            v0 += (cy0 - y0) * fv;
            v1 -= (y1 - cy1) * fv;
        }
        self.vert(cx0, cy0, u0, v0, c, rect, param);
        self.vert(cx1, cy0, u1, v0, c, rect, param);
        self.vert(cx1, cy1, u1, v1, c, rect, param);
        self.vert(cx0, cy0, u0, v0, c, rect, param);
        self.vert(cx1, cy1, u1, v1, c, rect, param);
        self.vert(cx0, cy1, u0, v1, c, rect, param);
    }

    pub fn solid(&mut self, r: RectF, c: Color) {
        self.quad(r.x, r.y, r.right(), r.bottom(), [0.0; 4], c, [0.0; 4], [0.0, 0.0, 0.0, MODE_SOLID]);
    }

    /// SDF rounded rect fill. `feather` ~1 = crisp edge; large = soft glow.
    pub fn rrect(&mut self, r: RectF, radius: f32, c: Color, feather: f32) {
        let e = feather + 1.0;
        let rect = [r.x + r.w / 2.0, r.y + r.h / 2.0, r.w / 2.0, r.h / 2.0];
        self.quad(
            r.x - e,
            r.y - e,
            r.right() + e,
            r.bottom() + e,
            [0.0; 4],
            c,
            rect,
            [radius, feather.max(0.5), 0.0, MODE_SDF],
        );
    }

    /// SDF rounded rect outline.
    pub fn border(&mut self, r: RectF, radius: f32, width: f32, c: Color) {
        let e = 2.0;
        let rect = [r.x + r.w / 2.0, r.y + r.h / 2.0, r.w / 2.0, r.h / 2.0];
        self.quad(
            r.x - e,
            r.y - e,
            r.right() + e,
            r.bottom() + e,
            [0.0; 4],
            c,
            rect,
            [radius, 0.7, width, MODE_SDF],
        );
    }

    /// Neon halo behind a rect.
    pub fn glow(&mut self, r: RectF, radius: f32, c: Color, spread: f32) {
        self.rrect(r, radius + spread * 0.4, c, spread);
    }

    /// Full-screen scanline overlay.
    pub fn scanlines(&mut self, w: f32, h: f32, alpha: f32) {
        self.quad(
            0.0,
            0.0,
            w,
            h,
            [0.0; 4],
            [0.0, 0.0, 0.0, alpha],
            [0.0; 4],
            [0.0, 0.0, 0.0, MODE_SCAN],
        );
    }

}

mod text;
