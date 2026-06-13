//! Pixel-space GPU UI for the browser: SDF shapes, proportional text,
//! animation. The terminal keeps the cell-grid renderer; this module is the
//! browser's "video game" front-end over the same App state machine.

pub mod anim;
pub mod atlas;
pub mod draw;
pub mod emoji;
pub mod render;
pub mod theme;
pub mod view;
