use serde_json::json;

use super::calls::{parse_tool_calls, ToolCall};
use super::exec::format_code_search;
use crate::github::{CodeMatch, CodeSearch};

#[test]
fn parses_code_search_calls() {
    let content = json!([
        {"type": "tool_use", "id": "t1", "name": "code_search",
         "input": {"query": "LIVE_GEN language:rust", "repo": "owner/name", "page": 3}},
        {"type": "tool_use", "id": "t2", "name": "code_search",
         "input": {"query": "wrap_chars"}},
    ]);
    let calls = parse_tool_calls(&content);
    assert_eq!(calls.len(), 2);
    match &calls[0] {
        ToolCall::CodeSearch { id, query, repo, page } => {
            assert_eq!(id, "t1");
            assert_eq!(query, "LIVE_GEN language:rust");
            assert_eq!(repo.as_deref(), Some("owner/name"));
            assert_eq!(*page, 3);
        }
        _ => panic!("expected CodeSearch"),
    }
    // Defaults: no repo scope, page 1.
    match &calls[1] {
        ToolCall::CodeSearch { repo, page, .. } => {
            assert_eq!(*repo, None);
            assert_eq!(*page, 1);
        }
        _ => panic!("expected CodeSearch"),
    }
    assert_eq!(calls[0].label(), "search LIVE_GEN language:rust in owner/name");
}

#[test]
fn formats_code_search_results() {
    let res = CodeSearch {
        total: 271,
        incomplete: false,
        items: vec![
            CodeMatch {
                repo: "a/b".into(),
                path: "src/x.rs".into(),
                lines: vec!["fn live_gen() {".into(), "use LIVE_GEN;".into()],
            },
            CodeMatch { repo: "c/d".into(), path: "lib/y.ts".into(), lines: vec![] },
        ],
    };
    let out = format_code_search(&res, 1);
    assert!(out.starts_with("271 matching files · page 1\n"), "{}", out);
    assert!(out.contains("a/b src/x.rs\n  > fn live_gen() {\n  > use LIVE_GEN;\n"), "{}", out);
    assert!(out.contains("c/d lib/y.ts\n"), "{}", out);
    // 271 > 100 (one page): pagination hint present.
    assert!(out.contains("more pages exist"), "{}", out);

    // Page 3 of 271 (last page — 300 ≥ 271): no hint; incomplete flag = ~.
    let res = CodeSearch { total: 271, incomplete: true, items: vec![CodeMatch {
        repo: "a/b".into(), path: "z".into(), lines: vec![] }] };
    let out = format_code_search(&res, 3);
    assert!(out.starts_with("~271 matching files · page 3\n"), "{}", out);
    assert!(!out.contains("more pages exist"), "{}", out);
}

#[test]
fn empty_results_guide_the_agent() {
    let res = CodeSearch { total: 0, incomplete: false, items: vec![] };
    let out = format_code_search(&res, 1);
    assert!(out.contains("no matches"), "{}", out);
}

#[test]
fn only_non_get_github_calls_are_mutating() {
    let content = json!([
        {"type": "tool_use", "id": "g", "name": "github_api",
         "input": {"method": "GET", "path": "/user"}},
        {"type": "tool_use", "id": "p", "name": "github_api",
         "input": {"method": "post", "path": "/repos/o/n/issues", "body": {"title": "x"}}},
        {"type": "tool_use", "id": "d", "name": "github_api",
         "input": {"method": "DELETE", "path": "/repos/o/n/git/refs/heads/x"}},
        {"type": "tool_use", "id": "b", "name": "bash", "input": {"command": "ls /"}},
    ]);
    let calls = parse_tool_calls(&content);
    // GET reads, bash over the VFS, and search never mutate; POST/PUT/PATCH/
    // DELETE github_api calls do (case-insensitively).
    assert!(!calls[0].is_mutating(), "GET must not gate");
    assert!(calls[1].is_mutating(), "lowercase post must gate");
    assert!(calls[2].is_mutating(), "DELETE must gate");
    assert!(!calls[3].is_mutating(), "bash is VFS-only, never mutates");
}
