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

const RUST_KW: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
    "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
    "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true",
    "type", "unsafe", "use", "where", "while",
];
const JS_KW: &[&str] = &[
    "async", "await", "break", "case", "catch", "class", "const", "continue", "default",
    "delete", "do", "else", "enum", "export", "extends", "false", "finally", "for", "function",
    "if", "implements", "import", "in", "instanceof", "interface", "let", "new", "null", "of",
    "private", "public", "readonly", "return", "static", "super", "switch", "this", "throw",
    "true", "try", "type", "typeof", "undefined", "var", "void", "while", "yield",
];
const PY_KW: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
    "else", "except", "False", "finally", "for", "from", "global", "if", "import", "in", "is",
    "lambda", "None", "not", "or", "pass", "raise", "return", "True", "try", "while", "with",
    "yield",
];
const GO_KW: &[&str] = &[
    "break", "case", "chan", "const", "continue", "default", "defer", "else", "fallthrough",
    "false", "for", "func", "go", "goto", "if", "import", "interface", "map", "nil", "package",
    "range", "return", "select", "struct", "switch", "true", "type", "var",
];
const DATA_KW: &[&str] = &["true", "false", "null"];

static RUST: LangSpec = LangSpec {
    line_comments: &["//"],
    block_comment: Some(("/*", "*/")),
    string_delims: &['"'],
    keywords: RUST_KW,
    md_headers: false,
};
static JS: LangSpec = LangSpec {
    line_comments: &["//"],
    block_comment: Some(("/*", "*/")),
    string_delims: &['"', '\'', '`'],
    keywords: JS_KW,
    md_headers: false,
};
static PY: LangSpec = LangSpec {
    line_comments: &["#"],
    block_comment: None,
    string_delims: &['"', '\''],
    keywords: PY_KW,
    md_headers: false,
};
static GO: LangSpec = LangSpec {
    line_comments: &["//"],
    block_comment: Some(("/*", "*/")),
    string_delims: &['"', '`'],
    keywords: GO_KW,
    md_headers: false,
};
static C: LangSpec = LangSpec {
    line_comments: &["//"],
    block_comment: Some(("/*", "*/")),
    string_delims: &['"', '\''],
    keywords: &[
        "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else",
        "enum", "extern", "float", "for", "goto", "if", "int", "long", "register", "return",
        "short", "signed", "sizeof", "static", "struct", "switch", "typedef", "union",
        "unsigned", "void", "volatile", "while",
    ],
    md_headers: false,
};
static SHELL: LangSpec = LangSpec {
    line_comments: &["#"],
    block_comment: None,
    string_delims: &['"', '\''],
    keywords: &[
        "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac",
        "function", "in", "return", "local", "export",
    ],
    md_headers: false,
};
static JSON: LangSpec = LangSpec {
    line_comments: &[],
    block_comment: None,
    string_delims: &['"'],
    keywords: DATA_KW,
    md_headers: false,
};
static TOML: LangSpec = LangSpec {
    line_comments: &["#"],
    block_comment: None,
    string_delims: &['"', '\''],
    keywords: DATA_KW,
    md_headers: false,
};
static MD: LangSpec = LangSpec {
    line_comments: &[],
    block_comment: None,
    string_delims: &['`'],
    keywords: &[],
    md_headers: true,
};

pub fn lang_for_path(path: &str) -> Option<&'static LangSpec> {
    let ext = path.rsplit('.').next()?.to_ascii_lowercase();
    match ext.as_str() {
        "rs" => Some(&RUST),
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" => Some(&JS),
        "py" => Some(&PY),
        "go" => Some(&GO),
        "c" | "h" | "cpp" | "hpp" | "cc" | "java" | "cs" => Some(&C),
        "sh" | "bash" | "zsh" => Some(&SHELL),
        "json" => Some(&JSON),
        "toml" | "yml" | "yaml" | "ini" | "cfg" => Some(&TOML),
        "md" | "markdown" => Some(&MD),
        _ => None,
    }
}

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
                    spans.push((i.saturating_sub(0), end + close.chars().count(), theme::SYN_COMMENT));
                    // The whole region from line start to here was comment if
                    // we entered the line inside one; merge handled by caller
                    // ordering since we push from position i.
                    if i > 0 {
                        // also cover the prefix we walked over
                    }
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
