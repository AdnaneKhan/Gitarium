//! Pipeline execution, command dispatch, and shared file-input helpers.

use crate::vfs;

use super::words::extract_redirects;
use super::{cmds_filter, cmds_fs, cmds_text, jq, parse, HELP};

pub(super) fn run_pipeline(cmd: &str) -> Result<String, String> {
    let segs = parse::split_pipes(cmd)?;
    let n = segs.len();
    let mut stdin = String::new();
    for (i, seg) in segs.iter().enumerate() {
        let (mut args, rin, rout) = extract_redirects(seg)?;
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
                let ok =
                    if append { vfs::append(&path, &result) } else { vfs::write(&path, result) };
                if !ok {
                    return Err(format!("{}: read-only (knowledge module)", path));
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
pub(super) fn read_file(path: &str) -> Result<String, String> {
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
pub(super) fn input(files: &[String], stdin: &str) -> Result<String, String> {
    if files.is_empty() {
        return Ok(stdin.to_string());
    }
    let mut out = String::new();
    for f in files {
        out.push_str(&read_file(f)?);
    }
    Ok(out)
}

/// Component-aware prefix test: `dir` covers itself and the paths beneath
/// it (`/r1` covers `/r1` and `/r1/x`, but not `/r1.json` or `/r10.json`).
pub(super) fn under_dir(path: &str, dir: &str) -> bool {
    dir == "/"
        || path
            .strip_prefix(dir)
            .map(|rest| rest.is_empty() || rest.starts_with('/'))
            .unwrap_or(false)
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
        "ls" => cmds_fs::ls(args),
        "head" | "tail" => cmds_filter::head_tail(name, args, stdin),
        "grep" => cmds_filter::grep(args, stdin),
        "wc" => cmds_text::wc(args, stdin),
        "sort" => cmds_text::sort_cmd(args, stdin),
        "uniq" => cmds_text::uniq(args, stdin),
        "cut" => cmds_text::cut(args, stdin),
        "find" => cmds_fs::find(args),
        "rm" => cmds_fs::rm(args),
        "jq" => jq::jq_cmd(args, stdin),
        // Directories are implicit in the VFS — accept these as no-ops so
        // `mkdir /x && echo y > /x/f` works instead of derailing.
        "mkdir" => Ok(String::new()),
        "touch" => {
            for f in args.iter().filter(|a| !a.starts_with('-')) {
                if !vfs::exists(f) && !vfs::write(f, String::new()) {
                    return Err(format!("touch: {}: read-only (knowledge module)", f));
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
