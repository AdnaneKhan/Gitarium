//! Building the agent transcript's wrapped display lines: word wrap,
//! hyperlink detection, code-fence detection with syntax highlighting, and
//! the source-line ids that let copies reconstruct logical text.

use super::links::{wrap_links, LinkSpan};
use super::text::lang_for_tag;
use super::*;

pub(super) struct TLine {
    pub(super) text: String,
    pub(super) color: Color,
    pub(super) label: bool,
    pub(super) code: bool,
    pub(super) spans: Vec<highlight::Span>,
    /// Clickable hyperlink ranges over this wrapped line's display chars.
    pub(super) links: Vec<LinkSpan>,
    /// Logical source-line id; wrapped segments of one source line share
    /// it. None = decoration (label / separator).
    pub(super) src: Option<u32>,
}

/// Wrapped transcript lines plus the distinct hyperlink targets they
/// reference (indexed by each line's `links`).
pub(super) fn build_transcript(transcript: &[AgentItem], cols: usize) -> (Vec<TLine>, Vec<String>) {
    let deco = |text: String, color: Color| TLine {
        text,
        color,
        label: false,
        code: false,
        spans: Vec::new(),
        links: Vec::new(),
        src: None,
    };
    let mut lines: Vec<TLine> = Vec::new();
    let mut urls: Vec<String> = Vec::new();
    let mut next_src: u32 = 0;
    // Prose path: detect links, then wrap to the mono column budget.
    let push_wrapped =
        |lines: &mut Vec<TLine>, next_src: &mut u32, urls: &mut Vec<String>, text: &str, color: Color| {
            for raw in text.split('\n') {
                let src = *next_src;
                *next_src += 1;
                for (seg, links) in wrap_links(raw, cols, urls) {
                    lines.push(TLine {
                        text: seg,
                        color,
                        label: false,
                        code: false,
                        spans: Vec::new(),
                        links,
                        src: Some(src),
                    });
                }
            }
        };
    // Assistant text: ``` fences become syntax-highlighted code blocks.
    let push_assistant =
        |lines: &mut Vec<TLine>, next_src: &mut u32, urls: &mut Vec<String>, t: &str| {
            let mut in_code = false;
            let mut spec: Option<&'static highlight::LangSpec> = None;
            let mut state = LineState::Normal;
            for raw in t.split('\n') {
                let trimmed = raw.trim_start();
                // Line-anchored fences: any ``` line opens (first info-string
                // token is the language); only a bare ``` closes, so stray
                // backticks inside a block stay content.
                if trimmed.starts_with("```") && (!in_code || trimmed[3..].trim().is_empty()) {
                    in_code = !in_code;
                    if in_code {
                        spec = lang_for_tag(trimmed[3..].split_whitespace().next().unwrap_or(""));
                        state = LineState::Normal;
                    }
                    continue;
                }
                if !in_code {
                    push_wrapped(lines, next_src, urls, raw, with_a(TEXT, 0.92));
                    continue;
                }
                let expanded = raw.replace('\r', "").replace('\t', "    ");
                let (spans, next) = match spec {
                    Some(sp) => highlight::highlight(sp, &expanded, state),
                    None => (Vec::new(), state),
                };
                state = next;
                let src = *next_src;
                *next_src += 1;
                let chars: Vec<char> = expanded.chars().collect();
                let mut s0 = 0;
                loop {
                    let s1 = (s0 + cols).min(chars.len());
                    let seg_spans = spans
                        .iter()
                        .filter(|(a, b, _)| *b > s0 && *a < s1)
                        .map(|(a, b, c)| (a.saturating_sub(s0), (b - s0).min(s1 - s0), *c))
                        .collect();
                    lines.push(TLine {
                        text: chars[s0..s1].iter().collect(),
                        color: with_a(TEXT, 0.9),
                        label: false,
                        code: true,
                        spans: seg_spans,
                        links: Vec::new(),
                        src: Some(src),
                    });
                    s0 = s1;
                    if s0 >= chars.len() {
                        break;
                    }
                }
            }
        };
    for item in transcript {
        match item {
            AgentItem::User(t) => {
                lines.push(TLine {
                    text: "YOU".into(),
                    color: CYAN,
                    label: true,
                    code: false,
                    spans: Vec::new(),
                    links: Vec::new(),
                    src: None,
                });
                push_wrapped(&mut lines, &mut next_src, &mut urls, t, TEXT);
            }
            AgentItem::Text(t) => {
                lines.push(TLine {
                    text: "AGENT".into(),
                    color: MAGENTA,
                    label: true,
                    code: false,
                    spans: Vec::new(),
                    links: Vec::new(),
                    src: None,
                });
                push_assistant(&mut lines, &mut next_src, &mut urls, t);
            }
            AgentItem::Tool { label, done } => {
                let (icon, color) = match done {
                    None => ('●', YELLOW),
                    Some(true) => ('✓', GREEN),
                    Some(false) => ('✗', RED),
                };
                push_wrapped(&mut lines, &mut next_src, &mut urls, &format!("{} {}", icon, label), with_a(color, 0.85));
            }
            AgentItem::Error(e) => push_wrapped(&mut lines, &mut next_src, &mut urls, &format!("✗ {}", e), RED),
        }
        lines.push(deco(String::new(), TEXT));
    }
    (lines, urls)
}
