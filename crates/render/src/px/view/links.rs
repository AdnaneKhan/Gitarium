//! Hyperlink detection for transcript prose: markdown `[text](url)` (shows
//! the text) and bare `http(s)://…` URLs (shown verbatim). Link spans are
//! carried through word-wrapping so each wrapped segment knows which of its
//! column ranges are clickable.

use super::text::wrap_chars;

/// A link range over a line's *display* chars: `[start, end)` columns plus the
/// index of the target URL in the shared url table.
pub(super) type LinkSpan = (usize, usize, usize);

/// Parse one logical line, then wrap it to `cols` columns. Returns each
/// wrapped segment paired with the link spans that fall inside it (columns
/// local to the segment). `urls` accumulates the distinct targets; the third
/// span field indexes into it.
pub(super) fn wrap_links(raw: &str, cols: usize, urls: &mut Vec<String>) -> Vec<(String, Vec<LinkSpan>)> {
    // Match wrap_chars' normalization up front so display-char offsets line up
    // with the wrapped segments (which would otherwise expand tabs under us).
    let norm = raw.replace('\r', "").replace('\t', "    ");
    let (disp, spans) = parse_links(&norm, urls);
    let mut segs = Vec::new();
    wrap_chars(&disp, cols, &mut segs);
    let mut out = Vec::with_capacity(segs.len());
    let mut base = 0usize; // display-char offset at the start of this segment
    for seg in segs {
        let len = seg.chars().count();
        let (s0, s1) = (base, base + len);
        let local: Vec<LinkSpan> = spans
            .iter()
            .filter_map(|&(a, b, u)| {
                let (a2, b2) = (a.max(s0), b.min(s1));
                (a2 < b2).then_some((a2 - s0, b2 - s0, u))
            })
            .collect();
        out.push((seg, local));
        base = s1;
    }
    out
}

/// Build a line's display string (markdown link text inlined, bare URLs kept
/// verbatim) together with the link spans over it.
fn parse_links(src: &str, urls: &mut Vec<String>) -> (String, Vec<LinkSpan>) {
    let chars: Vec<char> = src.chars().collect();
    let mut disp = String::new();
    let mut col = 0usize;
    let mut spans = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '[' {
            if let Some((text, url, next)) = md_link(&chars, i) {
                let idx = push_url(urls, &url);
                disp.extend(text.iter());
                spans.push((col, col + text.len(), idx));
                col += text.len();
                i = next;
                continue;
            }
        }
        if let Some(end) = bare_url(&chars, i) {
            let url: String = chars[i..end].iter().collect();
            let idx = push_url(urls, &url);
            disp.push_str(&url);
            spans.push((col, col + (end - i), idx));
            col += end - i;
            i = end;
            continue;
        }
        disp.push(chars[i]);
        col += 1;
        i += 1;
    }
    (disp, spans)
}

/// Bare `http(s)://` URL spans over a line's chars — no markdown, no display
/// rewrite — for contexts like the file viewer where the rendered text must
/// stay byte-identical to the source. Columns are char indices into `line`.
pub(super) fn url_spans(line: &str, urls: &mut Vec<String>) -> Vec<LinkSpan> {
    let chars: Vec<char> = line.chars().collect();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if let Some(end) = bare_url(&chars, i) {
            let url: String = chars[i..end].iter().collect();
            let idx = push_url(urls, &url);
            spans.push((i, end, idx));
            i = end;
        } else {
            i += 1;
        }
    }
    spans
}

/// `[text](url)` anchored at `open` (a `[`). Returns the text chars, the url,
/// and the index just past the closing `)`. None when the shape doesn't match
/// or either part is empty.
fn md_link(chars: &[char], open: usize) -> Option<(&[char], String, usize)> {
    let close = open + 1 + chars[open + 1..].iter().position(|&c| c == ']')?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let pstart = close + 2;
    let pend = pstart + chars[pstart..].iter().position(|&c| c == ')')?;
    let text = &chars[open + 1..close];
    let url: String = chars[pstart..pend].iter().collect();
    if text.is_empty() || url.is_empty() {
        return None;
    }
    Some((text, url, pend + 1))
}

/// End index (exclusive) of a bare `http://` / `https://` URL anchored at `i`,
/// with trailing sentence punctuation trimmed off. None when `i` isn't a URL.
pub(super) fn bare_url(chars: &[char], i: usize) -> Option<usize> {
    let rest = &chars[i..];
    let scheme = if starts_with(rest, "https://") {
        8
    } else if starts_with(rest, "http://") {
        7
    } else {
        return None;
    };
    let mut end = i;
    // Consume URL chars: stop at whitespace and at delimiters that wrap URLs
    // in markup — quotes, angle brackets, backtick, braces — so a URL inside
    // `href="…"` or `<…>` is captured cleanly, not run together with the tag.
    while end < chars.len() && !is_url_boundary(chars[end]) {
        end += 1;
    }
    // Trim punctuation that almost always belongs to the surrounding prose,
    // not the link — "see https://x.com." or "(https://x.com)".
    while end > i && matches!(chars[end - 1], '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}') {
        end -= 1;
    }
    (end > i + scheme).then_some(end)
}

/// Characters that terminate a bare URL: whitespace plus the delimiters that
/// commonly enclose URLs in HTML/markdown/code.
fn is_url_boundary(c: char) -> bool {
    c.is_whitespace() || matches!(c, '"' | '\'' | '`' | '<' | '>' | '\\' | '^' | '{' | '}' | '|')
}

fn starts_with(chars: &[char], pat: &str) -> bool {
    chars.len() >= pat.len() && chars.iter().zip(pat.chars()).all(|(&c, p)| c == p)
}

pub(super) fn push_url(urls: &mut Vec<String>, url: &str) -> usize {
    if let Some(i) = urls.iter().position(|u| u == url) {
        return i;
    }
    urls.push(url.to_string());
    urls.len() - 1
}
