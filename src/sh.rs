//! Minimal bash-like interpreter over the in-memory VFS — the agent's
//! navigation environment, in the spirit of vercel-labs' just-bash: a
//! from-scratch implementation of the commands agents actually use, with
//! no OS access. Supports pipes, `>` / `>>` / `<` redirects, `;` and `&&`
//! sequencing, and a small builtin set including full jq (via jaq).

use crate::vfs;

/// Cap on one bash invocation's output. Data is never lost to this cap —
/// it stays in the VFS for a narrower follow-up command.
const OUTPUT_LIMIT: usize = 8_000;

const HELP: &str = "commands:\n\
  ls [DIR]                 list files (sizes in chars)\n\
  cat FILE…                print files\n\
  head|tail [-n N]         first/last N lines (default 10)\n\
  grep [-i -n -v -c -r] PATTERN [FILE…]   regex line search (-r: all files)\n\
  wc [-l -w -c]            line/word/char counts\n\
  sort [-r -n -u]          sort lines\n\
  uniq [-c]                collapse adjacent duplicate lines\n\
  cut -d C -f LIST         split columns (e.g. -d, -f1,3)\n\
  find [DIR] -name GLOB    locate files by name (* and ?)\n\
  echo [-n] TEXT…          print text\n\
  rm FILE…                 delete files\n\
  mkdir, touch             accepted (directories are implicit)\n\
  jq [-r] FILTER [FILE]    full jq language; quote filters with ' '\n\
  pwd, help\n\
syntax: pipes a | b · redirects > >> < · sequencing ; and &&\n\
NOT supported: shell variables, $(…) substitution, glob expansion in\n\
arguments (use find -name or grep -r), cd, loops, ||, and any network or\n\
OS access — GitHub calls go through the github_api tool.\n\
files: API responses are saved as /rN.json — e.g. cat /r1.json | jq -r '.[].name' | head -20";

pub fn run(cmdline: &str) -> (String, bool) {
    let mut output = String::new();
    let mut all_ok = true;
    let mut prev_ok = true;
    for (cmd, needs_prev) in split_seq(cmdline) {
        let cmd = cmd.trim();
        if cmd.is_empty() {
            continue;
        }
        if needs_prev && !prev_ok {
            continue;
        }
        if let Err(e) = check_unsupported(cmd) {
            prev_ok = false;
            all_ok = false;
            output.push_str(&e);
            output.push('\n');
            continue;
        }
        match run_pipeline(cmd) {
            Ok(s) => {
                prev_ok = true;
                if !s.is_empty() {
                    output.push_str(&s);
                    if !s.ends_with('\n') {
                        output.push('\n');
                    }
                }
            }
            Err(e) => {
                prev_ok = false;
                all_ok = false;
                output.push_str(&e);
                output.push('\n');
            }
        }
    }
    if output.is_empty() {
        ("(no output)".to_string(), all_ok)
    } else {
        (cap(output), all_ok)
    }
}

/// Bound a result at OUTPUT_LIMIT with a pointer back to the (intact) VFS.
fn cap(output: String) -> String {
    let chars = output.chars().count();
    if chars <= OUTPUT_LIMIT {
        return output;
    }
    let mut s: String = output.chars().take(OUTPUT_LIMIT).collect();
    s.push_str(&format!(
        "\n…output truncated ({} chars total) — narrow with head, grep, or jq; the files are still in the VFS",
        chars
    ));
    s
}

// ---------------------------------------------------------------------------
// Dedicated tool entry points (same engines as the builtins, but inputs
// arrive structured — no shell quoting to trip over)
// ---------------------------------------------------------------------------

/// The `find` tool: locate files by name glob.
pub fn tool_find(path: Option<&str>, pattern: &str) -> (String, bool) {
    let mut args: Vec<String> = Vec::new();
    if let Some(p) = path {
        args.push(p.to_string());
    }
    args.push("-name".to_string());
    args.push(pattern.to_string());
    match find(&args) {
        Ok(s) if s.is_empty() => ("no files match".to_string(), true),
        Ok(s) => (cap(s), true),
        Err(e) => (e, false),
    }
}

/// The `grep` tool: regex search across file contents, `path:line:text`.
pub fn tool_grep(pattern: &str, path: Option<&str>, ignore_case: bool) -> (String, bool) {
    let mut args: Vec<String> = vec!["-n".to_string(), "-r".to_string()];
    if ignore_case {
        args.push("-i".to_string());
    }
    args.push(pattern.to_string());
    if let Some(p) = path {
        args.push(p.to_string());
    }
    match grep(&args, "") {
        Ok(s) => (cap(s), true),
        Err(e) if e == "grep: no matches" => ("no matches".to_string(), true),
        Err(e) => (e, false),
    }
}

/// Reject bash syntax this shell deliberately doesn't implement, with a
/// pointer to what works instead. Single-quoted text is exempt (that's
/// where jq's `$x` variables legitimately live).
fn check_unsupported(cmd: &str) -> Result<(), String> {
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
fn split_seq(s: &str) -> Vec<(String, bool)> {
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
fn split_pipes(s: &str) -> Result<Vec<String>, String> {
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

/// Pull `>`, `>>`, `<` (with or without an attached filename) out of an
/// argument list.
type Redirects = (Vec<String>, Option<String>, Option<(String, bool)>);
fn extract_redirects(toks: Vec<String>) -> Result<Redirects, String> {
    let mut args = Vec::new();
    let mut rin: Option<String> = None;
    let mut rout: Option<(String, bool)> = None;
    let mut i = 0;
    while i < toks.len() {
        let t = &toks[i];
        let (op, rest) = if t == ">>" || t == ">" || t == "<" {
            (t.as_str(), "")
        } else if let Some(r) = t.strip_prefix(">>") {
            (">>", r)
        } else if t.len() > 1 && t.starts_with('>') {
            (">", &t[1..])
        } else if t.len() > 1 && t.starts_with('<') {
            ("<", &t[1..])
        } else {
            args.push(t.clone());
            i += 1;
            continue;
        };
        let target = if rest.is_empty() {
            i += 1;
            toks.get(i).cloned().ok_or(format!("'{}' needs a file", op))?
        } else {
            rest.to_string()
        };
        match op {
            "<" => rin = Some(target),
            ">" => rout = Some((target, false)),
            _ => rout = Some((target, true)),
        }
        i += 1;
    }
    Ok((args, rin, rout))
}

fn run_pipeline(cmd: &str) -> Result<String, String> {
    let segs = split_pipes(cmd)?;
    let n = segs.len();
    let mut stdin = String::new();
    for (i, seg) in segs.iter().enumerate() {
        let toks = shlex::split(seg).ok_or("unbalanced quotes")?;
        let (mut args, rin, rout) = extract_redirects(toks)?;
        if let Some(p) = rin {
            if i != 0 {
                return Err("'<' is only allowed on the first command".into());
            }
            stdin = read_file(&p)?;
        }
        if args.is_empty() {
            return Err("empty command".into());
        }
        let name = args.remove(0);
        let result = exec_cmd(&name, &args, &stdin)?;
        if i == n - 1 {
            if let Some((path, append)) = rout {
                if append {
                    vfs::append(&path, &result);
                } else {
                    vfs::write(&path, result);
                }
                return Ok(String::new());
            }
            return Ok(result);
        }
        if rout.is_some() {
            return Err("'>' is only allowed on the last command".into());
        }
        stdin = result;
    }
    Ok(stdin)
}

/// Read one file, with a self-correcting error when the path looks like an
/// unexpanded glob (this shell does not expand globs in arguments).
fn read_file(path: &str) -> Result<String, String> {
    vfs::read(path).ok_or_else(|| {
        if path.contains('*') || path.contains('?') {
            format!(
                "{}: no such file — globs are not expanded in arguments; use find -name or grep -r",
                path
            )
        } else {
            format!("{}: no such file", path)
        }
    })
}

/// Concatenate file arguments, or fall back to stdin.
fn input(files: &[String], stdin: &str) -> Result<String, String> {
    if files.is_empty() {
        return Ok(stdin.to_string());
    }
    let mut out = String::new();
    for f in files {
        out.push_str(&read_file(f)?);
    }
    Ok(out)
}

fn exec_cmd(name: &str, args: &[String], stdin: &str) -> Result<String, String> {
    match name {
        "echo" => {
            let (newline, rest) = match args.first().map(String::as_str) {
                Some("-n") => (false, &args[1..]),
                _ => (true, args),
            };
            Ok(format!("{}{}", rest.join(" "), if newline { "\n" } else { "" }))
        }
        "cat" => input(args, stdin),
        "pwd" => Ok("/\n".into()),
        "help" => Ok(format!("{}\n", HELP)),
        "ls" => ls(args),
        "head" | "tail" => head_tail(name, args, stdin),
        "grep" => grep(args, stdin),
        "wc" => wc(args, stdin),
        "sort" => sort_cmd(args, stdin),
        "uniq" => uniq(args, stdin),
        "cut" => cut(args, stdin),
        "find" => find(args),
        "rm" => rm(args),
        "jq" => jq_cmd(args, stdin),
        // Directories are implicit in the VFS — accept these as no-ops so
        // `mkdir /x && echo y > /x/f` works instead of derailing.
        "mkdir" => Ok(String::new()),
        "touch" => {
            for f in args.iter().filter(|a| !a.starts_with('-')) {
                if !vfs::exists(f) {
                    vfs::write(f, String::new());
                }
            }
            Ok(String::new())
        }
        "sed" | "awk" | "tr" => Err(format!("{}: not available — use grep, cut, or jq", name)),
        "xargs" | "tee" => Err(format!("{}: not available — use pipes and > redirects", name)),
        "curl" | "wget" | "git" | "gh" => Err(format!(
            "{}: this shell has no network or OS access — call the GitHub API with the github_api tool",
            name
        )),
        "cd" => Err("cd: not supported — the working directory is always / (use absolute paths)".into()),
        "python" | "python3" | "node" | "sh" | "bash" => {
            Err(format!("{}: no interpreters here — jq covers data transformation", name))
        }
        other => Err(format!("{}: command not found (run 'help' for the full command list)", other)),
    }
}

fn ls(args: &[String]) -> Result<String, String> {
    let dir = vfs::norm(args.first().map(String::as_str).unwrap_or("/"));
    let prefix = if dir == "/" { "/".to_string() } else { format!("{}/", dir) };
    let mut entries: Vec<(String, Option<usize>)> = Vec::new();
    for (path, size) in vfs::list() {
        if path == dir {
            entries.push((path, Some(size)));
            continue;
        }
        let Some(rest) = path.strip_prefix(&prefix) else { continue };
        match rest.split_once('/') {
            // Subdirectory: list once, no size.
            Some((d, _)) => {
                let d = format!("{}/", d);
                if entries.last().map(|(n, _)| n != &d).unwrap_or(true) {
                    entries.push((d, None));
                }
            }
            None => entries.push((rest.to_string(), Some(size))),
        }
    }
    if entries.is_empty() {
        return Ok(String::new());
    }
    Ok(entries
        .iter()
        .map(|(n, s)| match s {
            Some(s) => format!("{:>9}  {}", s, n),
            None => format!("{:>9}  {}", "-", n),
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n")
}

fn head_tail(name: &str, args: &[String], stdin: &str) -> Result<String, String> {
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

fn grep(args: &[String], stdin: &str) -> Result<String, String> {
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
            if path.starts_with(&prefix) || prefix == "/" {
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

fn wc(args: &[String], stdin: &str) -> Result<String, String> {
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

fn sort_cmd(args: &[String], stdin: &str) -> Result<String, String> {
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

fn uniq(args: &[String], stdin: &str) -> Result<String, String> {
    let counted = args.first().map(String::as_str) == Some("-c");
    let files: Vec<String> = args.iter().filter(|a| *a != "-c").cloned().collect();
    let text = input(&files, stdin)?;
    let mut out = String::new();
    let mut last: Option<&str> = None;
    let mut count = 0usize;
    let mut flush = |line: Option<&str>, count: usize, out: &mut String| {
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

fn cut(args: &[String], stdin: &str) -> Result<String, String> {
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

fn find(args: &[String]) -> Result<String, String> {
    let mut root = "/".to_string();
    let mut pattern: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-name" => {
                i += 1;
                pattern = Some(args.get(i).cloned().ok_or("-name needs a pattern")?);
            }
            p => root = vfs::norm(p),
        }
        i += 1;
    }
    let re = match &pattern {
        Some(g) => {
            let mut src = String::from("^");
            for ch in g.chars() {
                match ch {
                    '*' => src.push_str(".*"),
                    '?' => src.push('.'),
                    c if "\\.^$()[]{}+|".contains(c) => {
                        src.push('\\');
                        src.push(c);
                    }
                    c => src.push(c),
                }
            }
            src.push('$');
            Some(regex_lite::Regex::new(&src).map_err(|e| format!("find: bad pattern: {}", e))?)
        }
        None => None,
    };
    let mut out = String::new();
    for (path, _) in vfs::list() {
        if !(path.starts_with(&root) || root == "/") {
            continue;
        }
        let name = path.rsplit('/').next().unwrap_or(&path);
        if re.as_ref().map(|r| r.is_match(name)).unwrap_or(true) {
            out.push_str(&path);
            out.push('\n');
        }
    }
    Ok(out)
}

fn rm(args: &[String]) -> Result<String, String> {
    let files: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if files.is_empty() {
        return Err("rm: missing file".into());
    }
    for f in files {
        if !vfs::remove(f) {
            return Err(format!("rm: {}: no such file", f));
        }
    }
    Ok(String::new())
}

// ---------------------------------------------------------------------------
// jq — full filter language via jaq
// ---------------------------------------------------------------------------

fn jq_cmd(args: &[String], stdin: &str) -> Result<String, String> {
    let mut raw = false;
    let mut filter: Option<String> = None;
    let mut files = Vec::new();
    for a in args {
        match a.as_str() {
            "-r" => raw = true,
            "-c" | "-C" | "-M" => {} // output is always compact/uncolored
            _ if filter.is_none() => filter = Some(a.clone()),
            _ => files.push(a.clone()),
        }
    }
    let filter = filter.ok_or("jq: missing filter")?;
    let text = input(&files, stdin)?;
    let mut out = String::new();
    for val in jq_eval(&filter, &text)? {
        match &val {
            jaq_json::Val::TStr(b) if raw => out.push_str(&String::from_utf8_lossy(b)),
            v => out.push_str(&v.to_string()),
        }
        out.push('\n');
    }
    Ok(out)
}

fn jq_eval(filter_src: &str, input_text: &str) -> Result<Vec<jaq_json::Val>, String> {
    use jaq_core::load::{Arena, File, Loader};
    use jaq_core::{data, unwrap_valr, Compiler, Ctx, Vars};
    use jaq_json::Val;

    let input = jaq_json::read::parse_single(input_text.trim().as_bytes())
        .map_err(|e| format!("jq: input is not JSON: {}", e))?;
    let program = File { code: filter_src, path: () };
    let loader = Loader::new(jaq_core::defs().chain(jaq_std::defs()).chain(jaq_json::defs()));
    let arena = Arena::default();
    let modules = loader.load(&arena, program).map_err(|errs| {
        let msgs: Vec<String> = errs.iter().map(|(_, e)| format!("{:?}", e)).collect();
        format!("jq: parse error in '{}': {}", filter_src, msgs.join("; "))
    })?;
    let filter = Compiler::default()
        .with_funs(jaq_core::funs().chain(jaq_std::funs()).chain(jaq_json::funs()))
        .compile(modules)
        .map_err(|errs| {
            let msgs: Vec<String> = errs.iter().map(|(_, e)| format!("{:?}", e)).collect();
            format!("jq: compile error: {}", msgs.join("; "))
        })?;
    let ctx = Ctx::<data::JustLut<Val>>::new(&filter.lut, Vars::new([]));
    let mut out = Vec::new();
    for v in filter.id.run((ctx, input)).map(unwrap_valr) {
        out.push(v.map_err(|e| format!("jq: {}", e))?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::vfs;

    #[test]
    fn pipes_and_redirects() {
        vfs::clear();
        let (out, ok) = run("echo hello world > /t.txt; cat /t.txt");
        assert!(ok, "{}", out);
        assert_eq!(out, "hello world\n");
        let (out, _) = run("echo more >> /t.txt && cat /t.txt | wc -l");
        assert_eq!(out, "2\n");
    }

    #[test]
    fn grep_and_head() {
        vfs::clear();
        vfs::write("/a.txt", "Alpha\nbeta\nALPHA again\ngamma\n".into());
        let (out, ok) = run("grep -i alpha /a.txt | wc -l");
        assert!(ok);
        assert_eq!(out, "2\n");
        let (out, _) = run("cat /a.txt | head -n 2");
        assert_eq!(out, "Alpha\nbeta\n");
        let (out, ok) = run("grep nomatch /a.txt");
        assert!(!ok, "{}", out);
    }

    #[test]
    fn jq_full_language() {
        vfs::clear();
        vfs::write("/r1.json", r#"[{"name":"x","n":2},{"name":"y","n":1}]"#.into());
        let (out, ok) = run("cat /r1.json | jq -r 'sort_by(.n) | .[].name'");
        assert!(ok, "{}", out);
        assert_eq!(out, "y\nx\n");
        let (out, ok) = run("jq 'map(.n) | add' /r1.json");
        assert!(ok, "{}", out);
        assert_eq!(out, "3\n");
        let (out, ok) = run("jq '.[] | select(.name == \"x\") | .n' /r1.json");
        assert!(ok, "{}", out);
        assert_eq!(out, "2\n");
    }

    #[test]
    fn sequencing_and_errors() {
        vfs::clear();
        let (out, ok) = run("rm /missing && echo never");
        assert!(!ok);
        assert!(!out.contains("never"));
        let (out, _) = run("rm /missing; echo still-runs");
        assert!(out.contains("still-runs"));
        let (out, ok) = run("frobnicate");
        assert!(!ok);
        assert!(out.contains("command not found"));
    }

    #[test]
    fn unsupported_syntax_gets_guidance() {
        vfs::clear();
        let (out, ok) = run("echo $(date)");
        assert!(!ok);
        assert!(out.contains("not supported"), "{}", out);
        let (out, ok) = run("echo $HOME");
        assert!(!ok);
        assert!(out.contains("variables are not supported"), "{}", out);
        // jq variables in single quotes are fine.
        vfs::write("/r1.json", r#"[1,2,3]"#.into());
        let (out, ok) = run("jq '. as $all | $all | length' /r1.json");
        assert!(ok, "{}", out);
        assert_eq!(out, "3\n");
        let (out, ok) = run("curl https://api.github.com");
        assert!(!ok);
        assert!(out.contains("github_api"), "{}", out);
        let (out, ok) = run("cat /r*.json");
        assert!(!ok);
        assert!(out.contains("globs are not expanded"), "{}", out);
    }

    #[test]
    fn dedicated_tool_entry_points() {
        use super::{tool_find, tool_grep};
        vfs::clear();
        vfs::write("/r1.json", "{\"msg\": \"Cost: $12\"}\nplain line\n".into());
        vfs::write("/notes/a.md", "TODO: check costs\n".into());
        // Pattern with $ — exactly what shell quoting would mangle.
        let (out, ok) = tool_grep(r"\$\d+", None, false);
        assert!(ok, "{}", out);
        assert!(out.contains("/r1.json:1:"), "{}", out);
        let (out, ok) = tool_grep("cost", None, true);
        assert!(ok);
        assert_eq!(out.lines().count(), 2, "{}", out);
        let (out, ok) = tool_grep("nothing-here", None, false);
        assert!(ok);
        assert_eq!(out, "no matches");
        let (out, ok) = tool_find(None, "*.json");
        assert!(ok);
        assert_eq!(out, "/r1.json\n");
        let (out, ok) = tool_find(Some("/notes"), "*");
        assert!(ok);
        assert_eq!(out, "/notes/a.md\n");
    }

    #[test]
    fn mkdir_touch_noop() {
        vfs::clear();
        let (out, ok) = run("mkdir /notes && echo plan > /notes/a.md && cat /notes/a.md");
        assert!(ok, "{}", out);
        assert_eq!(out, "plan\n");
        let (_, ok) = run("touch /empty.txt && cat /empty.txt | wc -c");
        assert!(ok);
    }

    #[test]
    fn ls_sort_uniq_cut_find() {
        vfs::clear();
        vfs::write("/r1.json", "{}".into());
        vfs::write("/notes/plan.md", "x".into());
        let (out, _) = run("ls");
        assert!(out.contains("r1.json") && out.contains("notes/"), "{}", out);
        let (out, _) = run("find -name '*.json'");
        assert_eq!(out, "/r1.json\n");
        let (out, _) = run("echo 'b\na\nb' | sort | uniq -c | sort -rn | head -1 | cut -d b -f1");
        assert!(out.trim().starts_with('2'), "{}", out);
    }
}
