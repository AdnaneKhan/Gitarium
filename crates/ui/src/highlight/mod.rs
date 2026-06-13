//! Small hand-rolled lexers for syntax highlighting. Token spans are
//! computed per line; the only cross-line state is "inside a block
//! comment", carried in LineState so edits re-highlight incrementally.

use crate::ui::grid::Rgb;
use crate::ui::theme;

pub struct LangSpec {
    pub line_comments: &'static [&'static str],
    pub block_comment: Option<(&'static str, &'static str)>,
    pub string_delims: &'static [char],
    pub keywords: &'static [&'static str],
    /// Markdown-style: color '#'-prefixed lines whole.
    pub md_headers: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LineState {
    Normal,
    InBlockComment,
}

pub type Span = (usize, usize, Rgb); // [start, end) in char indices

mod langs;

pub use langs::lang_for_path;


pub fn highlight(spec: &LangSpec, line: &str, state: LineState) -> (Vec<Span>, LineState) {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut spans: Vec<Span> = Vec::new();
    let mut st = state;
    let mut i = 0usize;

    if spec.md_headers && st == LineState::Normal {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            return (vec![(0, n, theme::SYN_FUNC)], LineState::Normal);
        }
        if trimmed.starts_with("```") {
            return (vec![(0, n, theme::SYN_COMMENT)], LineState::Normal);
        }
    }

    while i < n {
        if st == LineState::InBlockComment {
            let (_, close) = spec.block_comment.unwrap();
            match find_at(&chars, i, close) {
                Some(end) => {
                    // Comment runs from i through the close delimiter.
                    spans.push((i, end + close.chars().count(), theme::SYN_COMMENT));
                    i = end + close.chars().count();
                    st = LineState::Normal;
                }
                None => {
                    spans.push((i, n, theme::SYN_COMMENT));
                    return (spans, LineState::InBlockComment);
                }
            }
            continue;
        }

        let c = chars[i];

        // Line comment?
        let mut matched = false;
        for lc in spec.line_comments {
            if find_here(&chars, i, lc) {
                spans.push((i, n, theme::SYN_COMMENT));
                return (spans, LineState::Normal);
            }
        }
        // Block comment open?
        if let Some((open, close)) = spec.block_comment {
            if find_here(&chars, i, open) {
                let body_start = i;
                let after_open = i + open.chars().count();
                match find_at(&chars, after_open, close) {
                    Some(end) => {
                        let stop = end + close.chars().count();
                        spans.push((body_start, stop, theme::SYN_COMMENT));
                        i = stop;
                    }
                    None => {
                        spans.push((body_start, n, theme::SYN_COMMENT));
                        return (spans, LineState::InBlockComment);
                    }
                }
                continue;
            }
        }
        // String?
        if spec.string_delims.contains(&c) {
            let start = i;
            i += 1;
            while i < n {
                if chars[i] == '\\' {
                    i += 2;
                    continue;
                }
                if chars[i] == c {
                    i += 1;
                    break;
                }
                i += 1;
            }
            spans.push((start, i.min(n), theme::SYN_STRING));
            matched = true;
        } else if c.is_ascii_digit() {
            let start = i;
            while i < n && (chars[i].is_ascii_alphanumeric() || chars[i] == '.' || chars[i] == '_') {
                i += 1;
            }
            spans.push((start, i, theme::SYN_NUMBER));
            matched = true;
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            if spec.keywords.contains(&word.as_str()) {
                spans.push((start, i, theme::SYN_KEYWORD));
            } else if i < n && chars[i] == '(' {
                spans.push((start, i, theme::SYN_FUNC));
            } else if word.chars().next().map(|f| f.is_uppercase()).unwrap_or(false) {
                spans.push((start, i, theme::SYN_TYPE));
            }
            matched = true;
        }
        if !matched {
            i += 1;
        }
    }
    (spans, st)
}

fn find_here(chars: &[char], at: usize, needle: &str) -> bool {
    let nd: Vec<char> = needle.chars().collect();
    if at + nd.len() > chars.len() {
        return false;
    }
    chars[at..at + nd.len()] == nd[..]
}

fn find_at(chars: &[char], from: usize, needle: &str) -> Option<usize> {
    let nd: Vec<char> = needle.chars().collect();
    if nd.is_empty() || chars.len() < nd.len() {
        return None;
    }
    (from..=chars.len() - nd.len()).find(|&i| chars[i..i + nd.len()] == nd[..])
}
