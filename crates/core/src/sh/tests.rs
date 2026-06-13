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
fn jq_time_builtins_work_without_bundled_tzdb() {
    // Pins the slim-jiff vendored jaq-std (vendor/jaq-std): jq's UTC time
    // builtins must keep working with the IANA tzdb dropped. All UTC-based,
    // so no named-zone data is needed.
    vfs::clear();
    let (out, ok) = run("echo 1609459200 | jq -r 'todate'");
    assert!(ok, "{}", out);
    assert_eq!(out, "2021-01-01T00:00:00Z\n");
    let (out, ok) = run(r#"echo '"2021-01-01T00:00:00Z"' | jq 'fromdate'"#);
    assert!(ok, "{}", out);
    assert_eq!(out, "1609459200\n");
    let (out, ok) = run("echo 1609459200 | jq -r 'gmtime | strftime(\"%Y-%m-%d\")'");
    assert!(ok, "{}", out);
    assert_eq!(out, "2021-01-01\n");
}

#[test]
fn base64_command_encodes_and_decodes() {
    vfs::clear();
    let (out, ok) = run("echo -n hi | base64");
    assert!(ok, "{}", out);
    assert_eq!(out, "aGk=\n");
    // -d tolerates the newlines GitHub wraps `content` with: pull the field
    // with jq, pipe straight into base64 -d, no pre-cleaning needed. (The
    // trailing \n is run()'s display normalization, like cat — base64 -d
    // itself adds none.)
    vfs::write("/r1.json", "{\"content\":\"SGVsbG8s\\nIFdvcmxkIQ==\\n\"}".into());
    let (out, ok) = run("jq -r '.content' /r1.json | base64 -d");
    assert!(ok, "{}", out);
    assert_eq!(out, "Hello, World!\n");
    // --decode alias, decoding from a literal arg via echo.
    let (out, ok) = run("echo aGk= | base64 --decode");
    assert!(ok, "{}", out);
    assert_eq!(out, "hi\n");
}

#[test]
fn jq_base64_decode_handles_github_contents() {
    // The `base64` command is the primary decode path, but jq's @base64d
    // is a valid in-filter alternative — with one sharp edge: unlike the
    // base64 command, @base64d is strict and rejects the newlines GitHub
    // wraps `content` with, so it needs an explicit gsub. This pins that
    // quirk so the two paths' differing tolerance stays documented.
    vfs::clear();
    // {"content":"SGVsbG8s\nIFdvcmxkIQ==\n"} — "Hello, World!" wrapped.
    vfs::write("/r1.json", "{\"content\":\"SGVsbG8s\\nIFdvcmxkIQ==\\n\"}".into());
    // @base64d is strict — the embedded \n must be stripped first, or it
    // errors with "Invalid symbol 10". gsub does that in-filter.
    let (out, ok) = run("jq -r '.content | gsub(\"\\n\";\"\") | @base64d' /r1.json");
    assert!(ok, "{}", out);
    assert_eq!(out, "Hello, World!\n");
    // On already-clean base64 it decodes directly; @base64 encodes too.
    // (Input via stdin — this jq wrapper has no -n/null-input flag.)
    let (out, ok) = run("echo '\"hi\"' | jq -r '@base64 | @base64d'");
    assert!(ok, "{}", out);
    assert_eq!(out, "hi\n");
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

#[test]
fn quoted_redirects_are_literal() {
    vfs::clear();
    // Quoted operators are data, not redirects.
    let (out, ok) = run(r#"echo ">""#);
    assert!(ok, "{}", out);
    assert_eq!(out, ">\n");
    let (out, ok) = run("echo '>' '>>' '<'");
    assert!(ok, "{}", out);
    assert_eq!(out, "> >> <\n");
    // Backslash-escaped operators stay literal too.
    let (out, ok) = run(r"echo \> x");
    assert!(ok, "{}", out);
    assert_eq!(out, "> x\n");
    // Quoted '>' mid-word, double- and single-quoted.
    let (out, ok) = run(r#"echo a">"b c'>'d"#);
    assert!(ok, "{}", out);
    assert_eq!(out, "a>b c>d\n");
    // grep for a literal '>' (the motivating case).
    vfs::write("/h.txt", "a -> b\nplain\n".into());
    let (out, ok) = run(r#"grep ">" /h.txt"#);
    assert!(ok, "{}", out);
    assert_eq!(out, "a -> b\n");
    // None of the commands above wrote a file.
    assert_eq!(vfs::list().len(), 1, "{:?}", vfs::list());
}

#[test]
fn unquoted_redirects_still_work() {
    vfs::clear();
    // Separate-word and glued-to-filename forms.
    let (out, ok) = run("echo a >/g.txt; echo b >> /g.txt; cat /g.txt");
    assert!(ok, "{}", out);
    assert_eq!(out, "a\nb\n");
    // Glued input redirect; quoted filename target.
    let (out, ok) = run("cat </g.txt | wc -l; echo c > \"/q file.txt\"; cat '/q file.txt'");
    assert!(ok, "{}", out);
    assert_eq!(out, "2\nc\n");
    // Multiple output redirects: the last one wins (as before).
    let (out, ok) = run("echo x > /a.txt > /b.txt; cat /b.txt");
    assert!(ok, "{}", out);
    assert_eq!(out, "x\n");
    assert!(!vfs::exists("/a.txt"));
}

#[test]
fn recursive_prefix_is_component_aware() {
    vfs::clear();
    vfs::write("/r1.json", "needle\n".into());
    vfs::write("/r10.json", "needle\n".into());
    vfs::write("/r1/sub.txt", "needle\n".into());
    // /r1 covers /r1/… only — not /r1.json or /r10.json.
    let (out, ok) = run("grep -r -n needle /r1");
    assert!(ok, "{}", out);
    assert_eq!(out, "/r1/sub.txt:1:needle\n");
    // A file path still matches itself.
    let (out, ok) = run("grep -r needle /r1.json");
    assert!(ok, "{}", out);
    assert_eq!(out, "/r1.json:needle\n");
    // find honours the same component rule.
    let (out, ok) = run("find /r1 -name '*'");
    assert!(ok, "{}", out);
    assert_eq!(out, "/r1/sub.txt\n");
    // …and so does the structured grep tool.
    let (out, ok) = super::tool_grep("needle", Some("/r1"), false);
    assert!(ok, "{}", out);
    assert_eq!(out, "/r1/sub.txt:1:needle\n");
}
