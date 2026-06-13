//! Selection helpers for the markdown view: a row's plain copy text and its
//! per-character x offsets (measured from the area's left edge, with the same
//! fonts/sizes the row is drawn with) so the shared selection machinery can
//! hit-test and band it. Tables/decoration rows have no precise geometry and
//! fall back to uniform cells.

use super::*;

pub(super) fn row_text(row: &MdRow) -> String {
    match row {
        MdRow::Line { spans, .. } | MdRow::Code { spans, .. } => {
            spans.iter().map(|s| s.text.as_str()).collect()
        }
        MdRow::Table { cells, .. } => cells
            .iter()
            .map(|c| c.iter().map(|s| s.text.as_str()).collect::<String>())
            .collect::<Vec<_>>()
            .join("  "),
        MdRow::Rule | MdRow::Blank => String::new(),
    }
}

pub(super) fn row_xs(atlas: &Atlas, row: &MdRow, code_x: f32) -> Option<Vec<f32>> {
    let (spans, start) = match row {
        MdRow::Line { spans, indent, .. } => (spans, *indent),
        MdRow::Code { spans, .. } => (spans, code_x),
        _ => return None,
    };
    let mut xs = Vec::new();
    let mut x = start;
    for s in spans {
        let mut prev: Option<char> = None;
        for ch in s.text.chars() {
            if let Some(p) = prev {
                x += atlas.kern(s.font, s.px, p, ch);
            }
            xs.push(x);
            x += atlas.advance(s.font, s.px, ch);
            prev = Some(ch);
        }
    }
    xs.push(x);
    Some(xs)
}
