//! Line-transforming builtins: wc, sort, uniq, cut.

use super::exec::input;

pub(super) fn wc(args: &[String], stdin: &str) -> Result<String, String> {
    let mut mode: Option<char> = None;
    let mut files = Vec::new();
    for a in args {
        match a.as_str() {
            "-l" => mode = Some('l'),
            "-w" => mode = Some('w'),
            "-c" => mode = Some('c'),
            _ => files.push(a.clone()),
        }
    }
    let text = input(&files, stdin)?;
    let l = text.lines().count();
    let w = text.split_whitespace().count();
    let c = text.chars().count();
    Ok(match mode {
        Some('l') => format!("{}\n", l),
        Some('w') => format!("{}\n", w),
        Some('c') => format!("{}\n", c),
        _ => format!("{} {} {}\n", l, w, c),
    })
}

pub(super) fn sort_cmd(args: &[String], stdin: &str) -> Result<String, String> {
    let (mut rev, mut num, mut uniq) = (false, false, false);
    let mut files = Vec::new();
    for a in args {
        match a.as_str() {
            "-r" => rev = true,
            "-n" => num = true,
            "-u" => uniq = true,
            "-rn" | "-nr" => {
                rev = true;
                num = true;
            }
            _ => files.push(a.clone()),
        }
    }
    let text = input(&files, stdin)?;
    let mut lines: Vec<&str> = text.lines().collect();
    if num {
        lines.sort_by(|a, b| {
            let pa: f64 = a.trim().split_whitespace().next().and_then(|t| t.parse().ok()).unwrap_or(0.0);
            let pb: f64 = b.trim().split_whitespace().next().and_then(|t| t.parse().ok()).unwrap_or(0.0);
            pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        lines.sort();
    }
    if rev {
        lines.reverse();
    }
    if uniq {
        lines.dedup();
    }
    Ok(if lines.is_empty() { String::new() } else { lines.join("\n") + "\n" })
}

pub(super) fn uniq(args: &[String], stdin: &str) -> Result<String, String> {
    let counted = args.first().map(String::as_str) == Some("-c");
    let files: Vec<String> = args.iter().filter(|a| *a != "-c").cloned().collect();
    let text = input(&files, stdin)?;
    let mut out = String::new();
    let mut last: Option<&str> = None;
    let mut count = 0usize;
    let flush = |line: Option<&str>, count: usize, out: &mut String| {
        if let Some(l) = line {
            if counted {
                out.push_str(&format!("{:>4} {}\n", count, l));
            } else {
                out.push_str(l);
                out.push('\n');
            }
        }
    };
    for line in text.lines() {
        if last == Some(line) {
            count += 1;
        } else {
            flush(last, count, &mut out);
            last = Some(line);
            count = 1;
        }
    }
    flush(last, count, &mut out);
    Ok(out)
}

pub(super) fn cut(args: &[String], stdin: &str) -> Result<String, String> {
    let mut delim = '\t';
    let mut fields: Vec<usize> = Vec::new();
    let mut files = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-d" {
            i += 1;
            delim = args.get(i).and_then(|v| v.chars().next()).ok_or("-d needs a delimiter")?;
        } else if let Some(v) = a.strip_prefix("-d") {
            delim = v.chars().next().ok_or("-d needs a delimiter")?;
        } else if a == "-f" || a.starts_with("-f") {
            let spec = if a == "-f" {
                i += 1;
                args.get(i).cloned().ok_or("-f needs fields")?
            } else {
                a[2..].to_string()
            };
            for part in spec.split(',') {
                match part.split_once('-') {
                    Some((a, b)) => {
                        let (a, b): (usize, usize) = (
                            a.parse().map_err(|_| "bad field range")?,
                            b.parse().map_err(|_| "bad field range")?,
                        );
                        fields.extend(a..=b);
                    }
                    None => fields.push(part.parse().map_err(|_| "bad field number")?),
                }
            }
        } else {
            files.push(a.clone());
        }
        i += 1;
    }
    if fields.is_empty() {
        return Err("cut: -f is required".into());
    }
    let text = input(&files, stdin)?;
    let mut out = String::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split(delim).collect();
        let picked: Vec<&str> =
            fields.iter().filter_map(|&f| cols.get(f.saturating_sub(1)).copied()).collect();
        out.push_str(&picked.join(&delim.to_string()));
        out.push('\n');
    }
    Ok(out)
}
