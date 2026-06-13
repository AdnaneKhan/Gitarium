//! Plain-text helpers: fence language tags and mono-column wrapping.

use crate::highlight;

/// Map a code-fence language tag to a highlighter spec via its usual file
/// extension.
pub(super) fn lang_for_tag(tag: &str) -> Option<&'static highlight::LangSpec> {
    let ext = match tag.to_ascii_lowercase().as_str() {
        "" => return None,
        "rust" => "rs",
        "python" => "py",
        "javascript" | "node" => "js",
        "typescript" => "ts",
        "golang" => "go",
        "shell" | "bash" | "zsh" | "console" => "sh",
        "yaml" => "yml",
        "markdown" => "md",
        "c++" => "cpp",
        t => return highlight::lang_for_path(&format!("f.{}", t)),
    };
    highlight::lang_for_path(&format!("f.{}", ext))
}

/// Wrap text to a mono-column budget: split on newlines, then soft-wrap
/// before words, hard-breaking words longer than one line. Tabs expand to
/// four spaces and CRs are dropped; every remaining char (indentation,
/// runs of spaces, whitespace-only lines) lands in exactly one segment, so
/// the segments of a source line concatenate back to it verbatim.
pub(super) fn wrap_chars(text: &str, cols: usize, out: &mut Vec<String>) {
    let cols = cols.max(1);
    for raw in text.split('\n') {
        let raw = raw.replace('\r', "").replace('\t', "    ");
        let chars: Vec<char> = raw.chars().collect();
        if chars.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut count = 0usize;
        let mut i = 0usize;
        while i < chars.len() {
            let start = i;
            let space = chars[i] == ' ';
            while i < chars.len() && (chars[i] == ' ') == space {
                i += 1;
            }
            let token = &chars[start..i];
            if space {
                // Spaces stay on the current line even past the budget;
                // trailing overflow draws nothing visible.
                line.extend(token);
                count += token.len();
                continue;
            }
            if count > 0 && count + token.len() > cols {
                out.push(std::mem::take(&mut line));
                count = 0;
            }
            for &ch in token {
                if count >= cols {
                    out.push(std::mem::take(&mut line));
                    count = 0;
                }
                line.push(ch);
                count += 1;
            }
        }
        out.push(line);
    }
}
