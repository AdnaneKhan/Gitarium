//! Cyberpunk HUD palette. Colors are linear-ish RGBA in 0..1.

use crate::ui::grid::Rgb;

pub type Color = [f32; 4];

pub const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a]
}

/// Convert a cell-theme Rgb (syntax colors etc.) to a px color.
pub fn c(rgb: Rgb, a: f32) -> Color {
    [rgb.0 as f32 / 255.0, rgb.1 as f32 / 255.0, rgb.2 as f32 / 255.0, a]
}

pub fn with_a(col: Color, a: f32) -> Color {
    [col[0], col[1], col[2], a]
}

// Backgrounds — near-black with a blue cast.
pub const BG0: Color = rgba(0x04, 0x07, 0x0d, 1.0); // page
pub const BG1: Color = rgba(0x09, 0x0e, 0x1a, 1.0); // panel
pub const BG2: Color = rgba(0x0f, 0x17, 0x27, 1.0); // raised / input

// Neon accents.
pub const CYAN: Color = rgba(0x00, 0xe5, 0xff, 1.0);
pub const MAGENTA: Color = rgba(0xff, 0x2b, 0xd6, 1.0);
pub const GREEN: Color = rgba(0x00, 0xff, 0x9c, 1.0);
pub const RED: Color = rgba(0xff, 0x3b, 0x5c, 1.0);
pub const YELLOW: Color = rgba(0xff, 0xc8, 0x57, 1.0);

// Text.
pub const TEXT: Color = rgba(0xd2, 0xe3, 0xf0, 1.0);
pub const DIM: Color = rgba(0x5d, 0x72, 0x90, 1.0);
pub const FAINT: Color = rgba(0x33, 0x42, 0x5c, 1.0);

// Lines.
pub const BORDER: Color = rgba(0x1b, 0x29, 0x40, 1.0);
pub const BORDER_BRIGHT: Color = rgba(0x2a, 0x3f, 0x63, 1.0);
