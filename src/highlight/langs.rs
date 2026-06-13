//! Per-language keyword tables and specs, plus extension mapping.

use super::LangSpec;

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
