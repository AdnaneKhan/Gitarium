//! Color-emoji support: codepoint classification, cluster detection, and
//! browser rasterization of emoji clusters into the RGBA color atlas. Emoji
//! are drawn with the OS's own color emoji font (nothing embedded → zero
//! bundle cost). Non-wasm builds have no rasterizer, so `color_glyph` returns
//! None and emoji fall back to the coverage atlas.

use super::atlas::{size_key, Atlas, Glyph, COLOR_ATLAS};

/// Emoji cell advance (~1.2em). Fixed per base codepoint so width math stays
/// consistent across `text`, `text_width`, and hit-testing regardless of the
/// rasterized bitmap's actual size.
pub fn emoji_cell(px: f32) -> f32 {
    px * 1.2
}

/// Zero-width emoji modifiers — consumed inside a cluster, never start one.
pub fn is_zero_width_emoji(ch: char) -> bool {
    matches!(ch, '\u{FE0F}' | '\u{FE0E}' | '\u{200D}' | '\u{1F3FB}'..='\u{1F3FF}')
}

pub fn is_regional(ch: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&ch)
}

/// The advance for an emoji-related codepoint (fixed cell for a base, half a
/// cell per regional indicator, zero for combining marks), or None for
/// ordinary text. Single source of truth so draw and measurement agree.
pub fn emoji_advance(px: f32, ch: char) -> Option<f32> {
    if is_zero_width_emoji(ch) {
        Some(0.0)
    } else if is_regional(ch) {
        Some(emoji_cell(px) * 0.5)
    } else if is_emoji(ch) {
        Some(emoji_cell(px))
    } else {
        None
    }
}

/// Whether `ch` should render through the color-emoji path: the astral emoji
/// planes plus a curated set of common BMP emoji that don't collide with the
/// UI's own symbol glyphs (✓ ✗ ⚠ ● ○ ◆ … ↑ ↓ stay coverage glyphs).
pub fn is_emoji(ch: char) -> bool {
    if is_zero_width_emoji(ch) {
        return false;
    }
    ch >= '\u{1F000}'
        || matches!(
            ch,
            '\u{2705}' | '\u{274C}' | '\u{2764}' | '\u{2B50}' | '\u{2728}'
                | '\u{26A1}' | '\u{2600}' | '\u{2601}' | '\u{2614}' | '\u{26D4}'
                | '\u{2714}' | '\u{2611}' | '\u{203C}' | '\u{2049}' | '\u{2757}'
                | '\u{2753}' | '\u{2795}' | '\u{2796}' | '\u{27A1}' | '\u{2B06}'
                | '\u{2B07}' | '\u{2B05}' | '\u{2B1B}' | '\u{2B1C}'
        )
}

/// Char length of the emoji cluster starting at `i` (which must satisfy
/// `is_emoji`): a flag (two regional indicators) or a base plus its trailing
/// VS16 / skin-tone / ZWJ-joined emoji.
pub fn emoji_cluster_len(chars: &[char], i: usize) -> usize {
    let mut j = i + 1;
    if is_regional(chars[i]) {
        if chars.get(j).is_some_and(|&c| is_regional(c)) {
            j += 1;
        }
        return j - i;
    }
    while let Some(&c) = chars.get(j) {
        if !is_zero_width_emoji(c) {
            break;
        }
        let zwj = c == '\u{200D}';
        j += 1;
        if zwj && chars.get(j).is_some_and(|&n| is_emoji(n)) {
            j += 1;
        }
    }
    j - i
}

impl Atlas {
    /// Rasterize an emoji cluster into the color atlas (cached per size +
    /// cluster), returning its glyph (uv into the color texture). None when no
    /// rasterizer is available (non-web) or the atlas is full.
    #[cfg(target_arch = "wasm32")]
    pub fn color_glyph(&mut self, px: f32, cluster: &str) -> Option<Glyph> {
        let key = (size_key(px), cluster.to_string());
        if let Some(g) = self.color_cache.get(&key) {
            return *g;
        }
        let g = self.raster_emoji(px, cluster);
        self.color_cache.insert(key, g);
        g
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn color_glyph(&mut self, _px: f32, _cluster: &str) -> Option<Glyph> {
        let _ = (size_key, COLOR_ATLAS); // keep imports used on native
        None
    }

    #[cfg(target_arch = "wasm32")]
    fn raster_emoji(&mut self, px: f32, cluster: &str) -> Option<Glyph> {
        let size = px.round().max(8.0);
        let (cw, chh) = ((size * 1.5).ceil() as u32, (size * 1.5).ceil() as u32);
        let ctx = self.raster_ctx()?.clone();
        let canvas = ctx.canvas()?;
        canvas.set_width(cw);
        canvas.set_height(chh);
        ctx.set_font(&format!(
            "{}px \"Apple Color Emoji\",\"Segoe UI Emoji\",\"Noto Color Emoji\",sans-serif",
            size as u32
        ));
        ctx.set_text_baseline("alphabetic");
        ctx.set_fill_style_str("#fff");
        ctx.clear_rect(0.0, 0.0, cw as f64, chh as f64);
        let _ = ctx.fill_text(cluster, 0.0, size as f64);
        let img = ctx.get_image_data(0.0, 0.0, cw as f64, chh as f64).ok()?;
        self.pack_color(&img.data().0, cw, chh, size)
    }

    /// Lazily build the offscreen 2D context used for emoji rasterization.
    #[cfg(target_arch = "wasm32")]
    fn raster_ctx(&mut self) -> Option<&web_sys::CanvasRenderingContext2d> {
        use wasm_bindgen::JsCast;
        if self.raster_ctx.is_none() {
            let canvas: web_sys::HtmlCanvasElement = web_sys::window()?
                .document()?
                .create_element("canvas")
                .ok()?
                .dyn_into()
                .ok()?;
            let ctx: web_sys::CanvasRenderingContext2d =
                canvas.get_context("2d").ok()??.dyn_into().ok()?;
            self.raster_ctx = Some(ctx);
        }
        self.raster_ctx.as_ref()
    }

    /// Copy an RGBA cluster bitmap into the color atlas; baseline-relative top.
    #[cfg(target_arch = "wasm32")]
    fn pack_color(&mut self, rgba: &[u8], w: u32, h: u32, baseline: f32) -> Option<Glyph> {
        if self.color_cur_x + w + 1 > COLOR_ATLAS {
            self.color_cur_x = 1;
            self.color_cur_y += self.color_row_h + 1;
            self.color_row_h = 0;
        }
        if self.color_cur_y + h + 1 > COLOR_ATLAS {
            return None; // atlas full
        }
        for row in 0..h {
            let src = (row * w * 4) as usize;
            let dst = (((self.color_cur_y + row) * COLOR_ATLAS + self.color_cur_x) * 4) as usize;
            self.color_pixels[dst..dst + (w * 4) as usize]
                .copy_from_slice(&rgba[src..src + (w * 4) as usize]);
        }
        let ts = COLOR_ATLAS as f32;
        let g = Glyph {
            uv: [
                self.color_cur_x as f32 / ts,
                self.color_cur_y as f32 / ts,
                (self.color_cur_x + w) as f32 / ts,
                (self.color_cur_y + h) as f32 / ts,
            ],
            left: 0.0,
            top: -baseline,
            w: w as f32,
            h: h as f32,
            advance: emoji_cell(baseline),
        };
        self.color_cur_x += w + 1;
        self.color_row_h = self.color_row_h.max(h);
        self.color_dirty = true;
        Some(g)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_cluster_advance() {
        // Astral + curated BMP emoji route to the color path; UI glyphs don't.
        assert!(is_emoji('🚀') && is_emoji('✅') && is_emoji('❌'));
        assert!(!is_emoji('✓') && !is_emoji('⚠') && !is_emoji('a') && !is_emoji('\u{FE0F}'));
        assert!(is_regional('🇺') && is_zero_width_emoji('\u{1F3FD}'));
        // Flags (two regionals) and base+skin-tone are single clusters.
        assert_eq!(emoji_cluster_len(&"🇺🇸!".chars().collect::<Vec<_>>(), 0), 2);
        assert_eq!(emoji_cluster_len(&"👍🏽 ".chars().collect::<Vec<_>>(), 0), 2);
        // Emoji advance by a fixed cell; combining marks are zero-width.
        assert_eq!(emoji_advance(20.0, '🚀'), Some(emoji_cell(20.0)));
        assert_eq!(emoji_advance(20.0, '\u{FE0F}'), Some(0.0));
        assert_eq!(emoji_advance(20.0, 'a'), None);
        let mut atlas = Atlas::new().unwrap();
        assert_eq!(atlas.advance(0, 20.0, '🚀'), emoji_cell(20.0));
        assert!(atlas.color_glyph(20.0, "🚀").is_none()); // no rasterizer on native
    }
}
