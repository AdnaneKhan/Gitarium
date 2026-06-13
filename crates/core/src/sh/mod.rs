//! Minimal bash-like interpreter over the in-memory VFS — the agent's
//! navigation environment, in the spirit of vercel-labs' just-bash: a
//! from-scratch implementation of the commands agents actually use, with
//! no OS access. Supports pipes, `>` / `>>` / `<` redirects, `;` and `&&`
//! sequencing, and a small builtin set including full jq (via jaq).

mod cmds_filter;
mod cmds_fs;
mod cmds_text;
mod exec;
mod jq;
mod parse;
#[cfg(test)]
mod tests;
mod words;

/// Cap on one bash invocation's output. Data is never lost to this cap —
/// it stays in the VFS for a narrower follow-up command.
const OUTPUT_LIMIT: usize = 8_000;

pub(super) const HELP: &str = "commands:\n\
  ls [DIR]                 list files (sizes in chars)\n\
  cat FILE…                print files\n\
  head|tail [-n N]         first/last N lines (default 10)\n\
  grep [-i -n -v -c -r] PATTERN [FILE…]   regex line search (-r: all files)\n\
  wc [-l -w -c]            line/word/char counts\n\
  sort [-r -n -u]          sort lines\n\
  uniq [-c]                collapse adjacent duplicate lines\n\
  cut -d C -f LIST         split columns (e.g. -d, -f1,3)\n\
  base64 [-d] [FILE…]      encode, or -d to decode (ignores newlines)\n\
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
    for (cmd, needs_prev) in parse::split_seq(cmdline) {
        let cmd = cmd.trim();
        if cmd.is_empty() {
            continue;
        }
        if needs_prev && !prev_ok {
            continue;
        }
        if let Err(e) = parse::check_unsupported(cmd) {
            prev_ok = false;
            all_ok = false;
            output.push_str(&e);
            output.push('\n');
            continue;
        }
        match exec::run_pipeline(cmd) {
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
    match cmds_fs::find(&args) {
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
    match cmds_filter::grep(&args, "") {
        Ok(s) => (cap(s), true),
        Err(e) if e == "grep: no matches" => ("no matches".to_string(), true),
        Err(e) => (e, false),
    }
}
