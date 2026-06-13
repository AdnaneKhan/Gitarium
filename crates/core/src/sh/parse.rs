//! Top-level command-line structure: unsupported-syntax guidance, `;`/`&&`
//! sequencing, and pipe splitting — all quote-aware.

/// Reject bash syntax this shell deliberately doesn't implement, with a
/// pointer to what works instead. Single-quoted text is exempt (that's
/// where jq's `$x` variables legitimately live).
pub(super) fn check_unsupported(cmd: &str) -> Result<(), String> {
    let (mut sq, mut dq, mut esc) = (false, false, false);
    let cs: Vec<char> = cmd.chars().collect();
    for i in 0..cs.len() {
        let ch = cs[i];
        if esc {
            esc = false;
            continue;
        }
        match ch {
            '\\' if !sq => esc = true,
            '\'' if !dq => sq = !sq,
            '"' if !sq => dq = !dq,
            '`' if !sq => return Err("backticks/command substitution are not supported".into()),
            '$' if !sq => match cs.get(i + 1) {
                Some('(') => {
                    return Err("command substitution $(…) is not supported — run the commands separately".into())
                }
                Some(c) if c.is_alphanumeric() || *c == '_' || *c == '{' => {
                    return Err(
                        "shell variables are not supported (for jq variables like $x, single-quote the filter)".into(),
                    )
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(())
}

/// Split on top-level `;` and `&&`, respecting quotes. The bool says the
/// command only runs when the previous one succeeded.
pub(super) fn split_seq(s: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut and_next = false;
    let (mut sq, mut dq, mut esc) = (false, false, false);
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let ch = cs[i];
        if esc {
            cur.push(ch);
            esc = false;
            i += 1;
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
            ';' if !sq && !dq => {
                out.push((std::mem::take(&mut cur), and_next));
                and_next = false;
            }
            '&' if !sq && !dq && cs.get(i + 1) == Some(&'&') => {
                out.push((std::mem::take(&mut cur), and_next));
                and_next = true;
                i += 1;
            }
            _ => cur.push(ch),
        }
        i += 1;
    }
    out.push((cur, and_next));
    out
}

/// Split a command on top-level `|`, respecting quotes.
pub(super) fn split_pipes(s: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut sq, mut dq, mut esc) = (false, false, false);
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let ch = cs[i];
        if esc {
            cur.push(ch);
            esc = false;
            i += 1;
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
            '|' if !sq && !dq => {
                if cs.get(i + 1) == Some(&'|') {
                    return Err("'||' is not supported (use ';')".into());
                }
                out.push(std::mem::take(&mut cur));
            }
            _ => cur.push(ch),
        }
        i += 1;
    }
    out.push(cur);
    if out.iter().any(|s| s.trim().is_empty()) {
        return Err("empty command in pipeline".into());
    }
    Ok(out)
}
