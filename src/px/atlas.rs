//! Multi-font glyph atlas: Rajdhani (UI regular/bold) + JetBrains Mono,
//! rasterized on demand at arbitrary pixel sizes, shelf-packed into one
//! A8 texture. Glyph offsets are baseline-relative.

use std::collections::HashMap;

pub const ATLAS_SIZE: u32 = 2048;

pub const UI: u8 = 0;
pub const UI_BOLD: u8 = 1;
pub const MONO: u8 = 2;

const UI_BYTES: &[u8] = include_bytes!("../../assets/Rajdhani-Regular.ttf");
const UI_BOLD_BYTES: &[u8] = include_bytes!("../../assets/Rajdhani-Bold.ttf");
const MONO_BYTES: &[u8] = include_bytes!("../../assets/JetBrainsMono-Regular.ttf");

#[derive(Clone, Copy)]
pub struct Glyph {
    pub uv: [f32; 4],
    /// Offset from the pen position (x) and baseline (y, negative = up).
    pub left: f32,
    pub top: f32,
    pub w: f32,
    pub h: f32,
    pub advance: f32,
}

pub struct Atlas {
    fonts: Vec<fontdue::Font>,
    cache: HashMap<(u8, u16, char), Option<Glyph>>,
    pub pixels: Vec<u8>,
    cur_x: u32,
    cur_y: u32,
    row_h: u32,
    pub dirty: bool,
}

fn size_key(px: f32) -> u16 {
    (px * 2.0).round() as u16
}

impl Atlas {
    pub fn new() -> Result<Self, String> {
        let settings = fontdue::FontSettings::default();
        let fonts = vec![
            fontdue::Font::from_bytes(UI_BYTES, settings.clone()).map_err(|e| e.to_string())?,
            fontdue::Font::from_bytes(UI_BOLD_BYTES, settings.clone()).map_err(|e| e.to_string())?,
            fontdue::Font::from_bytes(MONO_BYTES, settings).map_err(|e| e.to_string())?,
        ];
        Ok(Atlas {
            fonts,
            cache: HashMap::new(),
            pixels: vec![0; (ATLAS_SIZE * ATLAS_SIZE) as usize],
            cur_x: 1,
            cur_y: 1,
            row_h: 0,
            dirty: true,
        })
    }

    /// (ascent, line_height) at the given size.
    pub fn metrics(&self, font: u8, px: f32) -> (f32, f32) {
        match self.fonts[font as usize].horizontal_line_metrics(px) {
            Some(m) => (m.ascent, m.new_line_size),
            None => (px * 0.8, px * 1.3),
        }
    }

    pub fn advance(&self, font: u8, px: f32, ch: char) -> f32 {
        self.fonts[font as usize].metrics(ch, px).advance_width
    }

    pub fn kern(&self, font: u8, px: f32, a: char, b: char) -> f32 {
        self.fonts[font as usize]
            .horizontal_kern(a, b, px)
            .unwrap_or(0.0)
    }

    pub fn glyph(&mut self, font: u8, px: f32, ch: char) -> Option<Glyph> {
        let key = (font, size_key(px), ch);
        if let Some(g) = self.cache.get(&key) {
            return *g;
        }
        let g = self.rasterize(font, px, ch);
        self.cache.insert(key, g);
        g
    }

    fn rasterize(&mut self, font: u8, px: f32, ch: char) -> Option<Glyph> {
        // Fallback chain: requested font → any embedded font with the
        // glyph (JetBrains Mono covers the symbols Rajdhani lacks) → '?'.
        let mut use_font = font as usize;
        let mut c = ch;
        if self.fonts[use_font].lookup_glyph_index(c) == 0 {
            match (0..self.fonts.len()).find(|&i| self.fonts[i].lookup_glyph_index(c) != 0) {
                Some(i) => use_font = i,
                None => {
                    c = '?';
                    use_font = font as usize;
                }
            }
        }
        let f = &self.fonts[use_font];
        let (m, bitmap) = f.rasterize(c, px);
        if m.width == 0 || m.height == 0 {
            return None;
        }
        let (w, h) = (m.width as u32, m.height as u32);
        if self.cur_x + w + 1 > ATLAS_SIZE {
            self.cur_x = 1;
            self.cur_y += self.row_h + 1;
            self.row_h = 0;
        }
        if self.cur_y + h + 1 > ATLAS_SIZE {
            return None; // atlas full — practically unreachable
        }
        for row in 0..h {
            let src = (row * w) as usize;
            let dst = ((self.cur_y + row) * ATLAS_SIZE + self.cur_x) as usize;
            self.pixels[dst..dst + w as usize].copy_from_slice(&bitmap[src..src + w as usize]);
        }
        let ts = ATLAS_SIZE as f32;
        let g = Glyph {
            uv: [
                self.cur_x as f32 / ts,
                self.cur_y as f32 / ts,
                (self.cur_x + w) as f32 / ts,
                (self.cur_y + h) as f32 / ts,
            ],
            left: m.xmin as f32,
            top: -((m.height as i32 + m.ymin) as f32),
            w: w as f32,
            h: h as f32,
            advance: m.advance_width,
        };
        self.cur_x += w + 1;
        self.row_h = self.row_h.max(h);
        self.dirty = true;
        Some(g)
    }
}
