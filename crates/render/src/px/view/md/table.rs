//! GFM table layout: measure natural column widths (header + body), pad, and
//! shrink proportionally to fit the available width. Emits one `MdRow::Table`
//! per source row (header first); cells are not wrapped — the draw pass clips
//! and aligns each within its column.

use super::inline::parse_inline;
use super::layout::{measure, mk_span, Base, MdSizes};
use super::*;

/// True when `line` is a GFM separator row (`|:--|--:|`).
pub(super) fn is_table_sep(line: &str) -> bool {
    let cells = split_cells(line);
    !cells.is_empty()
        && cells.iter().all(|c| {
            let c = c.trim();
            !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':') && c.contains('-')
        })
}

/// Split a row into trimmed cells, dropping the empty edges around
/// leading/trailing pipes; `\|` is a literal pipe.
fn split_cells(line: &str) -> Vec<String> {
    let mut cells: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'|') {
            cur.push('|');
            chars.next();
        } else if c == '|' {
            cells.push(cur.trim().to_string());
            cur = String::new();
        } else {
            cur.push(c);
        }
    }
    cells.push(cur.trim().to_string());
    if cells.first().is_some_and(|c| c.is_empty()) {
        cells.remove(0);
    }
    if cells.last().is_some_and(|c| c.is_empty()) {
        cells.pop();
    }
    cells
}

/// Parse a GFM table at `lines[0]` (header) / `lines[1]` (separator). Returns
/// the number of lines consumed.
pub(super) fn parse_table(lines: &[&str], urls: &mut Vec<String>, out: &mut Vec<Block>) -> usize {
    let header: Vec<String> = split_cells(lines[0]);
    let aligns: Vec<Align> = split_cells(lines[1])
        .iter()
        .map(|c| match (c.starts_with(':'), c.ends_with(':')) {
            (true, true) => Align::Center,
            (false, true) => Align::Right,
            _ => Align::Left,
        })
        .collect();
    let mut body = Vec::new();
    let mut used = 2;
    while used < lines.len() && lines[used].contains('|') && !lines[used].trim().is_empty() {
        body.push(split_cells(lines[used]).iter().map(|c| parse_inline(c, urls)).collect());
        used += 1;
    }
    out.push(Block::Table { aligns, header: header.iter().map(|c| parse_inline(c, urls)).collect(), body });
    used
}

#[allow(clippy::too_many_arguments)]
pub(super) fn layout(aligns: &[Align], header: &[Vec<Inline>], body: &[Vec<Vec<Inline>>], width: f32, sizes: &MdSizes, atlas: &Atlas, out: &mut Vec<MdRow>) {
    let ncols = header.len().max(body.iter().map(|r| r.len()).max().unwrap_or(0)).max(aligns.len());
    if ncols == 0 {
        return;
    }
    let head_base = Base { px: sizes.text_px, bold: true, color: CYAN };
    let cell_base = Base { px: sizes.text_px, bold: false, color: with_a(TEXT, 0.9) };
    let to_spans = |inl: &[Inline], base: &Base| -> Vec<Span> {
        inl.iter().map(|r| mk_span(r.text.clone(), r.style, sizes, base)).collect()
    };
    let span_w = |spans: &[Span]| -> f32 {
        spans.iter().map(|s| measure(atlas, s.font, s.px, &s.text)).sum()
    };
    let cell_of = |row: &[Vec<Inline>], c: usize, base: &Base| {
        to_spans(row.get(c).map(|v| v.as_slice()).unwrap_or(&[]), base)
    };

    let hcells: Vec<Vec<Span>> = (0..ncols).map(|c| cell_of(header, c, &head_base)).collect();
    let brows: Vec<Vec<Vec<Span>>> =
        body.iter().map(|row| (0..ncols).map(|c| cell_of(row, c, &cell_base)).collect()).collect();

    let pad = sizes.indent;
    let mut widths = vec![0.0f32; ncols];
    for (c, cell) in hcells.iter().enumerate() {
        widths[c] = widths[c].max(span_w(cell));
    }
    for row in &brows {
        for (c, cell) in row.iter().enumerate() {
            widths[c] = widths[c].max(span_w(cell));
        }
    }
    for w in widths.iter_mut() {
        *w += pad;
    }
    let total: f32 = widths.iter().sum();
    if total > width && total > 0.0 {
        let k = width / total;
        for w in widths.iter_mut() {
            *w *= k;
        }
    }

    out.push(MdRow::Table { cells: hcells, widths: widths.clone(), aligns: aligns.to_vec(), header: true });
    for row in brows {
        out.push(MdRow::Table { cells: row, widths: widths.clone(), aligns: aligns.to_vec(), header: false });
    }
}
