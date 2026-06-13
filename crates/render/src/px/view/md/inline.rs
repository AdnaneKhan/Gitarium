//! Inline markdown: emphasis, `code`, ~~strike~~, links, `<autolinks>`, bare
//! URLs, `:shortcode:` emoji, and backslash escapes (with CommonMark flanking
//! rules, so `snake_case` and `a * b` aren't emphasis).

use super::super::links::{bare_url, push_url};
use super::*;

pub(super) fn parse_inline(src: &str, urls: &mut Vec<String>) -> Vec<Inline> {
    let chars: Vec<char> = src.chars().collect();
    let mut out: Vec<Inline> = Vec::new();
    let mut buf = String::new();
    let mut st = Style::default();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\\' && chars.get(i + 1).is_some_and(|c| c.is_ascii_punctuation()) {
            buf.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if ch == ':' {
            if let Some((e, next)) = super::shortcode::expand_at(&chars, i) {
                flush(&mut out, &mut buf, st);
                out.push(Inline { text: e.into(), style: st });
                i = next;
                continue;
            }
        }
        if ch == '`' {
            let n = run_len(&chars, i, '`');
            if let Some(close) = find_run(&chars, i + n, '`', n) {
                flush(&mut out, &mut buf, st);
                out.push(Inline { text: code_text(&chars[i + n..close]), style: with_code(st) });
                i = close + n;
                continue;
            }
        }
        // Images are not rendered: show the alt text as a plain placeholder
        // (no link, no fetch), per the renderer's scope.
        if ch == '!' && chars.get(i + 1) == Some(&'[') {
            if let Some((text, _url, next)) = md_link(&chars, i + 1) {
                flush(&mut out, &mut buf, st);
                let alt = if text.trim().is_empty() { "[image]".to_string() } else { text };
                out.push(Inline { text: alt, style: st });
                i = next;
                continue;
            }
        }
        if ch == '[' {
            if let Some((text, url, next)) = md_link(&chars, i) {
                flush(&mut out, &mut buf, st);
                let idx = push_url(urls, &url);
                for mut run in parse_inline(&text, urls) {
                    run.style.link = Some(idx);
                    out.push(run);
                }
                i = next;
                continue;
            }
        }
        if ch == '<' {
            if let Some((url, next)) = autolink(&chars, i) {
                flush(&mut out, &mut buf, st);
                push_link(&mut out, urls, &url, st);
                i = next;
                continue;
            }
        }
        if ch == 'h' {
            if let Some(end) = bare_url(&chars, i) {
                flush(&mut out, &mut buf, st);
                let url: String = chars[i..end].iter().collect();
                push_link(&mut out, urls, &url, st);
                i = end;
                continue;
            }
        }
        if ch == '~' && chars.get(i + 1) == Some(&'~') {
            flush(&mut out, &mut buf, st);
            st.strike = !st.strike;
            i += 2;
            continue;
        }
        if ch == '*' || ch == '_' {
            let n = run_len(&chars, i, ch);
            if emphasis_ok(&chars, i, n, ch) {
                flush(&mut out, &mut buf, st);
                match n.min(3) {
                    1 => st.italic = !st.italic,
                    2 => st.bold = !st.bold,
                    _ => {
                        st.bold = !st.bold;
                        st.italic = !st.italic;
                    }
                }
                i += n.min(3);
                continue;
            }
        }
        buf.push(ch);
        i += 1;
    }
    flush(&mut out, &mut buf, st);
    out
}

fn flush(out: &mut Vec<Inline>, buf: &mut String, st: Style) {
    if !buf.is_empty() {
        out.push(Inline { text: std::mem::take(buf), style: st });
    }
}

fn with_code(mut st: Style) -> Style {
    st.code = true;
    st
}

fn push_link(out: &mut Vec<Inline>, urls: &mut Vec<String>, url: &str, st: Style) {
    let idx = push_url(urls, url);
    let mut s = st;
    s.link = Some(idx);
    out.push(Inline { text: url.to_string(), style: s });
}

/// Length of the run of `c` starting at `i`.
fn run_len(chars: &[char], i: usize, c: char) -> usize {
    let mut n = 0;
    while chars.get(i + n) == Some(&c) {
        n += 1;
    }
    n
}

/// Index of the start of the next run of exactly `n` `c`s at/after `from`.
fn find_run(chars: &[char], from: usize, c: char, n: usize) -> Option<usize> {
    let mut i = from;
    while i < chars.len() {
        if chars[i] == c {
            let len = run_len(chars, i, c);
            if len == n {
                return Some(i);
            }
            i += len;
        } else {
            i += 1;
        }
    }
    None
}

/// CommonMark: a code span wrapped in single spaces (with some non-space
/// content) drops one space each side.
fn code_text(c: &[char]) -> String {
    let strip = c.len() >= 2 && c[0] == ' ' && c[c.len() - 1] == ' ' && c.iter().any(|&x| x != ' ');
    if strip { c[1..c.len() - 1].iter().collect() } else { c.iter().collect() }
}

/// Whether a `*`/`_` run at `i` may act as an emphasis delimiter (flanking),
/// rejecting intra-word underscores so `snake_case` stays literal.
fn emphasis_ok(chars: &[char], i: usize, n: usize, ch: char) -> bool {
    let before = i.checked_sub(1).map(|j| chars[j]);
    let after = chars.get(i + n).copied();
    let left = after.is_some_and(|c| !c.is_whitespace());
    let right = before.is_some_and(|c| !c.is_whitespace());
    if !(left || right) {
        return false;
    }
    if ch == '_'
        && before.is_some_and(|c| c.is_alphanumeric())
        && after.is_some_and(|c| c.is_alphanumeric())
    {
        return false;
    }
    true
}

/// `[text](url)` at `open`; returns (text, url, index past `)`).
fn md_link(chars: &[char], open: usize) -> Option<(String, String, usize)> {
    let close = open + 1 + chars[open + 1..].iter().position(|&c| c == ']')?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let pstart = close + 2;
    let pend = pstart + chars[pstart..].iter().position(|&c| c == ')')?;
    let text: String = chars[open + 1..close].iter().collect();
    let url: String = chars[pstart..pend].iter().collect();
    if text.is_empty() || url.trim().is_empty() {
        return None;
    }
    Some((text, url.trim().to_string(), pend + 1))
}

/// `<http(s)://…>` autolink at `open`; returns (url, index past `>`).
fn autolink(chars: &[char], open: usize) -> Option<(String, usize)> {
    let end = open + 1 + chars[open + 1..].iter().position(|&c| c == '>')?;
    let url: String = chars[open + 1..end].iter().collect();
    (url.starts_with("http://") || url.starts_with("https://")).then(|| (url, end + 1))
}
