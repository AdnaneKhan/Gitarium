//! File-system builtins: ls, find, rm.

use crate::vfs;

use super::exec::under_dir;

pub(super) fn ls(args: &[String]) -> Result<String, String> {
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

pub(super) fn find(args: &[String]) -> Result<String, String> {
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
        if !under_dir(&path, &root) {
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

pub(super) fn rm(args: &[String]) -> Result<String, String> {
    let files: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if files.is_empty() {
        return Err("rm: missing file".into());
    }
    for f in files {
        if !vfs::remove(f) {
            return Err(if vfs::exists(f) {
                format!("rm: {}: read-only (knowledge module)", f)
            } else {
                format!("rm: {}: no such file", f)
            });
        }
    }
    Ok(String::new())
}
