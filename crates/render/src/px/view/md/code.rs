//! Fenced code-block layout: one `MdRow::Code` per source line, hard-wrapped
//! to the pixel width and syntax-highlighted (line-anchored state carried
//! across lines, matching the editor/transcript highlighter).

use super::layout::measure;
use super::*;

pub(super) fn rows(lang: &Option<&'static highlight::LangSpec>, lines: &[String], width: f32, sizes: &MdSizes, atlas: &Atlas, out: &mut Vec<MdRow>) {
    let cols = (width / measure(atlas, MONO, sizes.mono_px, "M")).floor().max(8.0) as usize;
    let start = out.len();
    let mut state = LineState::Normal;
    for raw in lines {
        let (spans, next) = match lang {
            Some(sp) => highlight::highlight(sp, raw, state),
            None => (Vec::new(), state),
        };
        state = next;
        let chars: Vec<char> = raw.chars().collect();
        let mut s0 = 0;
        loop {
            let s1 = (s0 + cols).min(chars.len());
            out.push(MdRow::Code { spans: code_spans(&chars[s0..s1], &spans, s0, sizes), first: false, last: false });
            s0 = s1;
            if s0 >= chars.len() {
                break;
            }
        }
    }
    if let Some(MdRow::Code { first, .. }) = out.get_mut(start) {
        *first = true;
    }
    if let Some(MdRow::Code { last, .. }) = out.last_mut() {
        *last = true;
    }
}

fn code_spans(seg: &[char], spans: &[highlight::Span], base: usize, sizes: &MdSizes) -> Vec<Span> {
    let default = with_a(TEXT, 0.9);
    let mk = |t: String, c: Color| Span { text: t, font: MONO, px: sizes.mono_px, color: c, code: false, strike: false, link: None };
    let mut out: Vec<Span> = Vec::new();
    let mut run = String::new();
    let mut run_color = default;
    for (k, &ch) in seg.iter().enumerate() {
        let abs = base + k;
        let color = spans
            .iter()
            .find(|(a, b, _)| *a <= abs && abs < *b)
            .map(|(_, _, c)| crate::px::theme::c(*c, 1.0))
            .unwrap_or(default);
        if !run.is_empty() && color != run_color {
            out.push(mk(std::mem::take(&mut run), run_color));
        }
        if run.is_empty() {
            run_color = color;
        }
        run.push(ch);
    }
    if !run.is_empty() {
        out.push(mk(run, run_color));
    }
    out
}
