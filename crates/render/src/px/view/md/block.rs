//! Block-level markdown: splits source into headings, paragraphs, lists,
//! blockquotes, fenced code, GFM tables, and thematic breaks. Line-based;
//! inline spans within each block come from `parse_inline`.

use super::super::text::lang_for_tag;
use super::inline::parse_inline;
use super::table;
use super::*;

pub(in crate::px::view) fn parse_blocks(src: &str, urls: &mut Vec<String>) -> Vec<Block> {
    let src = src.replace('\r', "");
    let lines: Vec<&str> = src.split('\n').collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim_start();
        if t.is_empty() {
            blocks.push(Block::Blank);
            i += 1;
        } else if let Some((fch, n)) = code_fence(t) {
            let lang = lang_for_tag(t[fch.len_utf8() * n..].split_whitespace().next().unwrap_or(""));
            let mut body = Vec::new();
            i += 1;
            while i < lines.len() && !closing_fence(lines[i], fch, n) {
                body.push(lines[i].replace('\t', "    "));
                i += 1;
            }
            i += (i < lines.len()) as usize; // consume the closing fence
            blocks.push(Block::Code(lang, body));
        } else if let Some((level, rest)) = atx_heading(t) {
            blocks.push(Block::Heading(level, parse_inline(rest, urls)));
            i += 1;
        } else if is_hr(t) {
            blocks.push(Block::Rule);
            i += 1;
        } else if lines[i].contains('|') && i + 1 < lines.len() && table::is_table_sep(lines[i + 1]) {
            i += table::parse_table(&lines[i..], urls, &mut blocks);
        } else if t.starts_with('>') {
            let depth = t.chars().take_while(|&c| c == '>' || c == ' ').filter(|&c| c == '>').count();
            let mut text = String::new();
            while i < lines.len() && lines[i].trim_start().starts_with('>') {
                let l = lines[i].trim_start().trim_start_matches(['>', ' ']);
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(l);
                i += 1;
            }
            blocks.push(Block::Quote(depth.min(4) as u8, parse_inline(&text, urls)));
        } else if let Some((marker, depth, task, content, used)) = list_item(&lines[i..]) {
            blocks.push(Block::Item { marker, depth, task, inl: parse_inline(&content, urls) });
            i += used;
        } else {
            let mut para = String::new();
            while i < lines.len() && !para_breaks(&lines, i) {
                if !para.is_empty() {
                    para.push(' ');
                }
                para.push_str(lines[i].trim());
                i += 1;
            }
            blocks.push(Block::Para(parse_inline(&para, urls)));
        }
    }
    blocks
}

/// True when line `i` cannot continue the current paragraph.
fn para_breaks(lines: &[&str], i: usize) -> bool {
    let t = lines[i].trim_start();
    t.is_empty()
        || code_fence(t).is_some()
        || atx_heading(t).is_some()
        || is_hr(t)
        || t.starts_with('>')
        || list_item(&lines[i..]).is_some()
        || (lines[i].contains('|') && i + 1 < lines.len() && table::is_table_sep(lines[i + 1]))
}

/// `(fence char, run length)` if `t` opens a ``` ``` / `~~~` fence.
fn code_fence(t: &str) -> Option<(char, usize)> {
    let ch = t.chars().next()?;
    if ch != '`' && ch != '~' {
        return None;
    }
    let n = t.chars().take_while(|&c| c == ch).count();
    (n >= 3).then_some((ch, n))
}

fn closing_fence(line: &str, ch: char, n: usize) -> bool {
    let t = line.trim();
    t.chars().take_while(|&c| c == ch).count() >= n && t.chars().all(|c| c == ch)
}

fn atx_heading(t: &str) -> Option<(u8, &str)> {
    let n = t.chars().take_while(|&c| c == '#').count();
    if n == 0 || n > 6 || t.chars().nth(n) != Some(' ') {
        return None;
    }
    Some((n as u8, t[n + 1..].trim().trim_end_matches(['#', ' '])))
}

fn is_hr(t: &str) -> bool {
    let s: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    s.len() >= 3 && ["-", "*", "_"].iter().any(|m| s.chars().all(|c| c == m.chars().next().unwrap()))
}

/// `(ordered marker, nesting depth, task state, content, lines consumed)`.
type ParsedItem = (Option<u64>, u8, Option<bool>, String, usize);

fn list_item(lines: &[&str]) -> Option<ParsedItem> {
    let exp = lines[0].replace('\t', "    ");
    let indent = exp.len() - exp.trim_start().len();
    let t = exp.trim_start();
    let (marker, rest) = if let Some(r) = ["- ", "* ", "+ "].iter().find_map(|m| t.strip_prefix(m)) {
        (None, r)
    } else {
        let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
        let after = t.get(digits..)?;
        if digits == 0 || !(after.starts_with(". ") || after.starts_with(") ")) {
            return None;
        }
        (Some(t[..digits].parse().ok()?), &after[2..])
    };
    let (task, body) = match ["[ ] ", "[x] ", "[X] "].iter().find(|m| rest.starts_with(**m)) {
        Some(m) => (Some(m.contains('x') || m.contains('X')), &rest[4..]),
        None => (None, rest),
    };
    let mut content = body.trim().to_string();
    let mut used = 1;
    while used < lines.len() && !lines[used].trim().is_empty() {
        let le = lines[used].replace('\t', "    ");
        if le.len() - le.trim_start().len() <= indent || list_item(&lines[used..]).is_some() {
            break;
        }
        content.push(' ');
        content.push_str(le.trim());
        used += 1;
    }
    Some((marker, (indent / 2).min(6) as u8, task, content, used))
}
