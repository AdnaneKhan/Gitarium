//! Canvas2D software backend for machines with no WebGL at all: interprets
//! the DrawList quad stream. Clipping already happened CPU-side and every
//! quad carries its primitive's parameters, so this is a straight decoder —
//! solids become fillRect, SDF rects become rounded paths (stroked when a
//! border width is set, shadow-blurred when the feather says glow), and
//! glyphs blit from the CPU atlas with per-run tinting (see glyphs.rs).

use wasm_bindgen::{Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

use super::super::atlas::{Atlas, ATLAS_SIZE, COLOR_ATLAS};
use super::super::draw::{DrawList, FLOATS_PER_VERT, MODE_EMOJI, MODE_GLYPH, MODE_SDF, MODE_SOLID};
use super::super::theme;
use super::glyphs::GlyphRun;

pub(super) struct State {
    pub(super) ctx: CanvasRenderingContext2d,
    pub(super) atlas_canvas: HtmlCanvasElement,
    atlas_ctx: CanvasRenderingContext2d,
    color_canvas: HtmlCanvasElement,
    color_ctx: CanvasRenderingContext2d,
    pub(super) scratch: HtmlCanvasElement,
    pub(super) scratch_ctx: CanvasRenderingContext2d,
}

fn ctx2d(canvas: &HtmlCanvasElement) -> Result<CanvasRenderingContext2d, String> {
    canvas
        .get_context("2d")
        .map_err(|e| format!("{:?}", e))?
        .ok_or("no 2d context")?
        .dyn_into()
        .map_err(|_| "2d context cast failed".to_string())
}

fn offscreen(w: u32, h: u32) -> Result<HtmlCanvasElement, String> {
    let c: HtmlCanvasElement = web_sys::window()
        .ok_or("no window")?
        .document()
        .ok_or("no document")?
        .create_element("canvas")
        .map_err(|e| format!("{:?}", e))?
        .dyn_into()
        .map_err(|_| "canvas cast failed".to_string())?;
    c.set_width(w);
    c.set_height(h);
    Ok(c)
}

pub(super) fn init(canvas: &HtmlCanvasElement) -> Result<State, String> {
    let ctx = ctx2d(canvas)?;
    let atlas_canvas = offscreen(ATLAS_SIZE, ATLAS_SIZE)?;
    let atlas_ctx = ctx2d(&atlas_canvas)?;
    let color_canvas = offscreen(COLOR_ATLAS, COLOR_ATLAS)?;
    let color_ctx = ctx2d(&color_canvas)?;
    let scratch = offscreen(512, 64)?;
    let scratch_ctx = ctx2d(&scratch)?;
    Ok(State { ctx, atlas_canvas, atlas_ctx, color_canvas, color_ctx, scratch, scratch_ctx })
}

pub(super) fn css(c: [f32; 4]) -> String {
    format!(
        "rgba({},{},{},{})",
        (c[0] * 255.0) as u8,
        (c[1] * 255.0) as u8,
        (c[2] * 255.0) as u8,
        c[3].clamp(0.0, 1.0)
    )
}

impl State {
    /// Coverage → white with alpha, so source-in tinting yields color.rgb
    /// at coverage × color.a (the same result the GL shader computes).
    fn upload_atlas(&self, atlas: &Atlas) {
        let mut rgba = vec![255u8; atlas.pixels.len() * 4];
        for (i, &a) in atlas.pixels.iter().enumerate() {
            rgba[i * 4 + 3] = a;
        }
        if let Ok(img) =
            ImageData::new_with_u8_clamped_array_and_sh(Clamped(&rgba), ATLAS_SIZE, ATLAS_SIZE)
        {
            let _ = self.atlas_ctx.put_image_data(&img, 0.0, 0.0);
        }
    }

    fn rrect_path(&self, cx: f32, cy: f32, hw: f32, hh: f32, radius: f32) {
        let (x, y, w, h) = ((cx - hw) as f64, (cy - hh) as f64, (hw * 2.0) as f64, (hh * 2.0) as f64);
        let r = (radius as f64).min(w / 2.0).min(h / 2.0).max(0.0);
        let ctx = &self.ctx;
        ctx.begin_path();
        ctx.move_to(x + r, y);
        let _ = ctx.arc_to(x + w, y, x + w, y + h, r);
        let _ = ctx.arc_to(x + w, y + h, x, y + h, r);
        let _ = ctx.arc_to(x, y + h, x, y, r);
        let _ = ctx.arc_to(x, y, x + w, y, r);
        ctx.close_path();
    }

    pub(super) fn flush(&mut self, atlas: &mut Atlas, dl: &DrawList, w: f32, h: f32) {
        if atlas.dirty {
            self.upload_atlas(atlas);
            atlas.dirty = false;
        }
        if atlas.color_dirty {
            super::glyphs::upload_color(&self.color_ctx, atlas);
            atlas.color_dirty = false;
        }
        let ctx = &self.ctx;
        let bg = theme::BG0;
        ctx.set_fill_style_str(&css([bg[0], bg[1], bg[2], 1.0]));
        ctx.fill_rect(0.0, 0.0, w as f64, h as f64);

        let stride = FLOATS_PER_VERT * 6;
        let v = &dl.verts;
        let mut run: Option<GlyphRun> = None;
        let mut base = 0;
        while base + stride <= v.len() {
            let q = &v[base..base + stride];
            base += stride;
            // Quad layout (see draw::quad): v0 = (x0,y0,u0,v0), the third
            // vertex is (x1,y1,u1,v1); color/rect/param ride every vertex.
            let (x0, y0) = (q[0], q[1]);
            let (x1, y1) = (q[32], q[33]);
            let color = [q[4], q[5], q[6], q[7]];
            let mode = q[15];

            // Glyph runs batch until a different primitive (or color)
            // arrives, preserving draw order.
            if mode != MODE_GLYPH {
                if let Some(r) = run.take() {
                    r.draw(self);
                }
            }
            if mode == MODE_GLYPH {
                let src = [
                    q[2] * ATLAS_SIZE as f32,
                    q[3] * ATLAS_SIZE as f32,
                    (q[34] - q[2]) * ATLAS_SIZE as f32,
                    (q[35] - q[3]) * ATLAS_SIZE as f32,
                ];
                let dst = [x0, y0, x1 - x0, y1 - y0];
                match &mut run {
                    Some(r) if r.color == color => r.push(src, dst),
                    _ => {
                        if let Some(r) = run.take() {
                            r.draw(self);
                        }
                        let mut r = GlyphRun::new(color);
                        r.push(src, dst);
                        run = Some(r);
                    }
                }
            } else if mode == MODE_EMOJI {
                super::glyphs::blit_emoji(ctx, &self.color_canvas, q, x0, y0, x1, y1);
            } else if mode == MODE_SOLID {
                ctx.set_fill_style_str(&css(color));
                ctx.fill_rect(x0 as f64, y0 as f64, (x1 - x0) as f64, (y1 - y0) as f64);
            } else if mode == MODE_SDF {
                let (cx, cy, hw, hh) = (q[8], q[9], q[10], q[11]);
                let (radius, feather, border_w) = (q[12], q[13], q[14]);
                if border_w > 0.0 {
                    // The GL band sits just inside the edge: stroke an
                    // inset path so the outer extent matches.
                    let inset = border_w / 2.0;
                    self.rrect_path(cx, cy, hw - inset, hh - inset, radius - inset);
                    ctx.set_line_width(border_w as f64);
                    ctx.set_stroke_style_str(&css(color));
                    ctx.stroke();
                } else if feather > 2.0 {
                    // Wide feather = glow: a shadow-blurred fill is the
                    // Canvas2D analogue of the SDF falloff.
                    self.rrect_path(cx, cy, hw, hh, radius);
                    ctx.set_shadow_color(&css(color));
                    ctx.set_shadow_blur(feather as f64);
                    ctx.set_fill_style_str(&css(color));
                    ctx.fill();
                    ctx.set_shadow_blur(0.0);
                    ctx.set_shadow_color("rgba(0,0,0,0)");
                } else {
                    self.rrect_path(cx, cy, hw, hh, radius);
                    ctx.set_fill_style_str(&css(color));
                    ctx.fill();
                }
            } else {
                // Scanlines: darken every other 2px band, like the shader.
                ctx.set_fill_style_str(&css([0.0, 0.0, 0.0, color[3]]));
                let mut y = y0;
                while y < y1 {
                    ctx.fill_rect(x0 as f64, y as f64, (x1 - x0) as f64, 2.0);
                    y += 4.0;
                }
            }
        }
        if let Some(r) = run.take() {
            r.draw(self);
        }
    }
}
