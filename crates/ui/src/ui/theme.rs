//! Shared color constants: syntax palette (GitHub dark) plus the status
//! colors used for workflow-run icons. The px renderer has its own HUD
//! palette in px/theme.rs.

use super::grid::Rgb;

pub const GREEN: Rgb = Rgb(0x3f, 0xb9, 0x50);
pub const RED: Rgb = Rgb(0xf8, 0x51, 0x49);
pub const YELLOW: Rgb = Rgb(0xd2, 0x99, 0x22);
pub const DIM: Rgb = Rgb(0x8b, 0x94, 0x9e);

pub const SYN_KEYWORD: Rgb = Rgb(0xff, 0x7b, 0x72);
pub const SYN_STRING: Rgb = Rgb(0xa5, 0xd6, 0xff);
pub const SYN_COMMENT: Rgb = Rgb(0x8b, 0x94, 0x9e);
pub const SYN_NUMBER: Rgb = Rgb(0x79, 0xc0, 0xff);
pub const SYN_FUNC: Rgb = Rgb(0xd2, 0xa8, 0xff);
pub const SYN_TYPE: Rgb = Rgb(0xff, 0xa6, 0x57);
