//! A self-contained Markdown renderer for the cyberpunk HUD. Parses a common
//! subset — ATX headings, paragraphs, unordered/ordered/task lists,
//! blockquotes, fenced code (syntax-highlighted), GFM pipe tables, thematic
//! breaks, and inline **bold** / *italic* / `code` / ~~strike~~ / links — into
//! drawable rows. Images, math, mermaid, and raw HTML are deliberately left
//! unrendered (shown as their source text), per the renderer's scope.
//!
//! Pipeline: `parse_blocks` (pure) → `layout_blocks` (measures + wraps to a
//! pixel width, needs the atlas) → `View::draw_markdown` (paints rows, emits
//! link hit-regions). The detail view is the primary consumer.

use super::*;

mod block;
mod code;
mod draw;
mod inline;
mod layout;
mod select;
mod shortcode;
mod table;

#[cfg(test)]
mod tests;

pub(super) use block::parse_blocks;
pub(super) use layout::{layout_blocks, MdSizes};


/// Inline character styling. `link` indexes a shared url table.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Style {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub strike: bool,
    pub link: Option<usize>,
}

/// A styled inline run from the inline parser.
pub(super) struct Inline {
    pub text: String,
    pub style: Style,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Align {
    Left,
    Center,
    Right,
}

/// A parsed block. `Blank` carries block separation as an explicit row so the
/// layout doesn't have to special-case spacing.
pub(super) enum Block {
    Heading(u8, Vec<Inline>),
    Para(Vec<Inline>),
    Code(Option<&'static highlight::LangSpec>, Vec<String>),
    Quote(u8, Vec<Inline>),
    /// A list item. `marker` = Some(n) for ordered (the number), None for a
    /// bullet; `depth` is the nesting level; `task` = checkbox state if any.
    Item {
        marker: Option<u64>,
        depth: u8,
        task: Option<bool>,
        inl: Vec<Inline>,
    },
    Table {
        aligns: Vec<Align>,
        header: Vec<Vec<Inline>>,
        body: Vec<Vec<Vec<Inline>>>,
    },
    Rule,
    Blank,
}

/// A measured, ready-to-draw span (one `dl.text` call's worth).
#[derive(Clone)]
pub(super) struct Span {
    pub text: String,
    pub font: u8,
    pub px: f32,
    pub color: Color,
    /// Draw a faint inline-code background behind this span.
    pub code: bool,
    pub strike: bool,
    pub link: Option<usize>,
}

/// Left-edge decoration for a laid-out line.
pub(super) enum Deco {
    None,
    /// A list marker ("•" or "1.") drawn just left of the text.
    Marker(String),
    Task(bool),
    /// Blockquote bars at the given nesting depth.
    Quote(u8),
    /// Heading level (h1/h2 get an underline rule).
    Heading(u8),
}

/// One drawable row. Rows are a uniform height so scrolling stays row-indexed
/// (see the detail view); blank rows provide block spacing.
pub(super) enum MdRow {
    Line { spans: Vec<Span>, indent: f32, deco: Deco },
    Code { spans: Vec<Span>, first: bool, last: bool },
    Table { cells: Vec<Vec<Span>>, widths: Vec<f32>, aligns: Vec<Align>, header: bool },
    Rule,
    Blank,
}
