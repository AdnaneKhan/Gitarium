//! Word-level lexing: raw (still-quoted) word splitting, shlex unquoting,
//! and redirect extraction that can still see the original quoting.

/// Split a pipeline segment into raw words at unquoted whitespace, keeping
/// quotes and escapes intact so redirect detection can still see them.
/// Mirrors shlex's lexing rules (whitespace set, word-start `#` comments)
/// so that `unquote` later yields exactly one token per word.
fn raw_words(s: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut sq, mut dq, mut esc, mut comment) = (false, false, false, false);
    for ch in s.chars() {
        if comment {
            comment = ch != '\n';
            continue;
        }
        if esc {
            cur.push(ch);
            esc = false;
            continue;
        }
        match ch {
            '\\' if !sq => {
                cur.push(ch);
                esc = true;
            }
            '\'' if !dq => {
                sq = !sq;
                cur.push(ch);
            }
            '"' if !sq => {
                dq = !dq;
                cur.push(ch);
            }
            '#' if !sq && !dq && cur.is_empty() => comment = true,
            ' ' | '\t' | '\n' if !sq && !dq => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(ch),
        }
    }
    if sq || dq || esc {
        return Err("unbalanced quotes".into());
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    Ok(out)
}

/// Strip one raw word's quoting and escapes (shlex semantics).
fn unquote(word: &str) -> Result<String, String> {
    let toks = shlex::split(word).ok_or("unbalanced quotes")?;
    Ok(toks.into_iter().next().unwrap_or_default())
}

/// Pull `>`, `>>`, `<` (with or without an attached filename) out of a raw
/// command segment. Operators are located before quote removal — a word can
/// only start at unquoted whitespace, so a word-initial `>`/`<` is a genuine
/// operator, while quoted or escaped ones (`">"`, `'>'`, `\>`, `a">"b`) stay
/// ordinary argument text.
pub(super) type Redirects = (Vec<String>, Option<String>, Option<(String, bool)>);

pub(super) fn extract_redirects(seg: &str) -> Result<Redirects, String> {
    let words = raw_words(seg)?;
    let mut args = Vec::new();
    let mut rin: Option<String> = None;
    let mut rout: Option<(String, bool)> = None;
    let mut i = 0;
    while i < words.len() {
        let w = words[i].as_str();
        let (op, rest) = if let Some(r) = w.strip_prefix(">>") {
            (">>", r)
        } else if let Some(r) = w.strip_prefix('>') {
            (">", r)
        } else if let Some(r) = w.strip_prefix('<') {
            ("<", r)
        } else {
            args.push(unquote(w)?);
            i += 1;
            continue;
        };
        let target = unquote(if rest.is_empty() {
            i += 1;
            words.get(i).map(String::as_str).ok_or(format!("'{}' needs a file", op))?
        } else {
            rest
        })?;
        match op {
            "<" => rin = Some(target),
            ">" => rout = Some((target, false)),
            _ => rout = Some((target, true)),
        }
        i += 1;
    }
    Ok((args, rin, rout))
}
