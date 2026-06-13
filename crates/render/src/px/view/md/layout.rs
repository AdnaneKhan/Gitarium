//! Layout: turn parsed blocks into drawable, width-wrapped rows. Measures
//! with the atlas (no `DrawList` needed) and emits uniform-height `MdRow`s so
//! the consumer can keep row-indexed scrolling. Tables are in `table`.

use super::{code, table};
use super::*;

/// Resolved (already DPR-scaled) sizes the layout draws at.
pub(in crate::px::view) struct MdSizes {
    pub text_px: f32,
    pub mono_px: f32,
    pub indent: f32,
    pub h_px: [f32; 6],
}

/// Base styling a block contributes before inline emphasis is applied.
pub(super) struct Base {
    pub px: f32,
    pub bold: bool,
    pub color: Color,
}

pub(in crate::px::view) fn layout_blocks(blocks: &[Block], width: f32, sizes: &MdSizes, atlas: &Atlas) -> Vec<MdRow> {
    let mut out = Vec::new();
    for b in blocks {
        match b {
            Block::Blank => out.push(MdRow::Blank),
            Block::Rule => out.push(MdRow::Rule),
            Block::Heading(level, inl) => {
                let base = Base {
                    px: sizes.h_px[(*level as usize - 1).min(5)],
                    bold: true,
                    color: if *level <= 2 { CYAN } else { with_a(TEXT, 0.95) },
                };
                emit(wrap(inl, width, atlas, sizes, &base), 0.0, Deco::Heading(*level), &mut out);
            }
            Block::Para(inl) => {
                let base = Base { px: sizes.text_px, bold: false, color: with_a(TEXT, 0.92) };
                emit(wrap(inl, width, atlas, sizes, &base), 0.0, Deco::None, &mut out);
            }
            Block::Quote(depth, inl) => {
                let indent = *depth as f32 * sizes.indent;
                let base = Base { px: sizes.text_px, bold: false, color: with_a(TEXT, 0.72) };
                emit(wrap(inl, (width - indent).max(40.0), atlas, sizes, &base), indent, Deco::Quote(*depth), &mut out);
            }
            Block::Item { marker, depth, task, inl } => {
                let indent = (*depth as f32 + 1.0) * sizes.indent;
                let base = Base { px: sizes.text_px, bold: false, color: with_a(TEXT, 0.92) };
                let deco = match task {
                    Some(done) => Deco::Task(*done),
                    None => Deco::Marker(marker.map_or("•".to_string(), |n| format!("{}.", n))),
                };
                emit(wrap(inl, (width - indent).max(40.0), atlas, sizes, &base), indent, deco, &mut out);
            }
            Block::Code(lang, lines) => code::rows(lang, lines, width, sizes, atlas, &mut out),
            Block::Table { aligns, header, body } => {
                table::layout(aligns, header, body, width, sizes, atlas, &mut out)
            }
        }
    }
    out
}

/// Push wrapped rows, giving only the first the block's decoration.
fn emit(rows: Vec<Vec<Span>>, indent: f32, deco: Deco, out: &mut Vec<MdRow>) {
    let mut deco = Some(deco);
    for spans in rows {
        out.push(MdRow::Line { spans, indent, deco: deco.take().unwrap_or(Deco::None) });
    }
}

pub(super) fn measure(atlas: &Atlas, font: u8, px: f32, s: &str) -> f32 {
    let mut w = 0.0;
    let mut prev: Option<char> = None;
    for ch in s.chars() {
        if let Some(p) = prev {
            w += atlas.kern(font, px, p, ch);
        }
        w += atlas.advance(font, px, ch);
        prev = Some(ch);
    }
    w
}

pub(super) fn mk_span(text: String, style: Style, sizes: &MdSizes, base: &Base) -> Span {
    if style.code {
        return Span { text, font: MONO, px: sizes.mono_px.min(base.px), color: with_a(CYAN, 0.92), code: true, strike: style.strike, link: style.link };
    }
    let bold = style.bold || base.bold;
    let color = if style.link.is_some() {
        CYAN
    } else if style.italic && !bold {
        with_a(TEXT, 0.80)
    } else {
        base.color
    };
    Span { text, font: if bold { UI_BOLD } else { UI }, px: base.px, color, code: false, strike: style.strike, link: style.link }
}

fn same(a: &Span, b: &Span) -> bool {
    a.font == b.font && a.px == b.px && a.color == b.color && a.code == b.code && a.strike == b.strike && a.link == b.link
}

/// Greedy word-wrap a styled inline sequence to `width`, returning rows of
/// merged spans. Always returns at least one (possibly empty) row.
fn wrap(inl: &[Inline], width: f32, atlas: &Atlas, sizes: &MdSizes, base: &Base) -> Vec<Vec<Span>> {
    let mut rows: Vec<Vec<Span>> = Vec::new();
    let mut cur: Vec<Span> = Vec::new();
    let mut x = 0.0;
    for run in inl {
        let chars: Vec<char> = run.text.chars().collect();
        let mut j = 0;
        while j < chars.len() {
            let space = chars[j] == ' ';
            let start = j;
            while j < chars.len() && (chars[j] == ' ') == space {
                j += 1;
            }
            let atom = mk_span(chars[start..j].iter().collect(), run.style, sizes, base);
            let w = measure(atlas, atom.font, atom.px, &atom.text);
            if !space && !cur.is_empty() && x + w > width {
                trim_trailing(&mut cur);
                rows.push(std::mem::take(&mut cur));
                x = 0.0;
            }
            if space && cur.is_empty() {
                continue; // drop leading spaces at a row start
            }
            x += w;
            match cur.last_mut() {
                Some(last) if same(last, &atom) => last.text.push_str(&atom.text),
                _ => cur.push(atom),
            }
        }
    }
    trim_trailing(&mut cur);
    rows.push(cur);
    rows
}

fn trim_trailing(spans: &mut Vec<Span>) {
    while let Some(last) = spans.last_mut() {
        let trimmed = last.text.trim_end_matches(' ');
        if trimmed.len() != last.text.len() {
            last.text.truncate(trimmed.len());
        }
        if last.text.is_empty() {
            spans.pop();
        } else {
            break;
        }
    }
}

