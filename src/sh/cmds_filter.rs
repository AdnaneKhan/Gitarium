//! Line-selecting builtins: grep (also backing the structured grep tool)
//! and head/tail.

use crate::vfs;

use super::exec::{input, read_file, under_dir};

pub(super) fn head_tail(name: &str, args: &[String], stdin: &str) -> Result<String, String> {
    let mut n: usize = 10;
    let mut files = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-n" {
            i += 1;
            n = args.get(i).and_then(|v| v.parse().ok()).ok_or("-n needs a number")?;
        } else if let Some(v) = a.strip_prefix("-n") {
            n = v.parse().map_err(|_| "-n needs a number")?;
        } else if let Some(v) = a.strip_prefix('-') {
            n = v.parse().map_err(|_| format!("bad flag '{}'", a))?;
        } else {
            files.push(a.clone());
        }
        i += 1;
    }
    let text = input(&files, stdin)?;
    let lines: Vec<&str> = text.lines().collect();
    let picked: Vec<&str> = if name == "head" {
        lines.iter().take(n).copied().collect()
    } else {
        lines.iter().skip(lines.len().saturating_sub(n)).copied().collect()
    };
    Ok(if picked.is_empty() { String::new() } else { picked.join("\n") + "\n" })
}

pub(super) fn grep(args: &[String], stdin: &str) -> Result<String, String> {
    let (mut icase, mut invert, mut count, mut nums, mut recursive) = (false, false, false, false, false);
    let mut pattern: Option<String> = None;
    let mut files = Vec::new();
    for a in args {
        match a.as_str() {
            "-i" => icase = true,
            "-v" => invert = true,
            "-c" => count = true,
            "-n" => nums = true,
            "-r" => recursive = true,
            "-in" | "-ni" => {
                icase = true;
                nums = true;
            }
            _ if pattern.is_none() => pattern = Some(a.clone()),
            _ => files.push(a.clone()),
        }
    }
    let pattern = pattern.ok_or("grep: missing pattern")?;
    let src = if icase { format!("(?i){}", pattern) } else { pattern.clone() };
    let re = regex_lite::Regex::new(&src).map_err(|e| format!("grep: bad pattern: {}", e))?;

    // Resolve sources: -r walks the VFS under an optional prefix.
    let mut sources: Vec<(Option<String>, String)> = Vec::new();
    if recursive {
        let prefix = vfs::norm(files.first().map(String::as_str).unwrap_or("/"));
        for (path, _) in vfs::list() {
            if under_dir(&path, &prefix) {
                if let Some(body) = vfs::read(&path) {
                    sources.push((Some(path), body));
                }
            }
        }
    } else if files.is_empty() {
        sources.push((None, stdin.to_string()));
    } else {
        let tag = files.len() > 1;
        for f in &files {
            let body = read_file(f).map_err(|e| format!("grep: {}", e))?;
            sources.push((tag.then(|| f.clone()), body));
        }
    }

    let mut out = String::new();
    let mut total = 0usize;
    for (label, body) in &sources {
        for (ln, line) in body.lines().enumerate() {
            if re.is_match(line) != invert {
                total += 1;
                if !count {
                    if let Some(l) = label {
                        out.push_str(l);
                        out.push(':');
                    }
                    if nums {
                        out.push_str(&format!("{}:", ln + 1));
                    }
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
    }
    if count {
        return Ok(format!("{}\n", total));
    }
    if total == 0 {
        return Err("grep: no matches".into());
    }
    Ok(out)
}
