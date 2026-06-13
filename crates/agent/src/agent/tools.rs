//! Tool schemas advertised to the model in every Messages request.

use serde_json::{json, Value};

pub(super) fn tools() -> Value {
    json!([
        {
            "name": "github_api",
            "description": "Execute one GitHub REST v3 API request, authenticated as the user. \
                Call this whenever you need to read or change anything on GitHub — repos, \
                issues, pull requests, file contents, branches, refs, actions, releases, \
                users, search. `path` starts with '/' and may include a query string, e.g. \
                /repos/OWNER/REPO/issues?state=open&per_page=100. `body` is the JSON request \
                body for POST/PUT/PATCH. Small responses come back inline; large JSON \
                responses are stored and you get an id plus a shape summary — inspect those \
                with query_response.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "method": {
                        "type": "string",
                        "enum": ["GET", "POST", "PUT", "PATCH", "DELETE"],
                        "description": "HTTP method"
                    },
                    "path": {
                        "type": "string",
                        "description": "API path starting with '/', optionally with a query string"
                    },
                    "body": {
                        "type": "object",
                        "description": "JSON request body, for POST/PUT/PATCH"
                    }
                },
                "required": ["method", "path"]
            }
        },
        {
            "name": "bash",
            "description": "Run a command in this session's minimal in-memory shell — use it to \
                navigate saved API responses and keep notes on long tasks. There is no real OS: \
                only a virtual filesystem holding the /rN.json files saved by github_api and \
                anything you write. The ONLY available commands are: ls, cat, head, tail, grep \
                (-i -n -v -c -r), wc (-l -w -c), sort (-r -n -u), uniq (-c), cut (-d -f), find \
                (-name), echo (-n), rm, mkdir, touch, pwd, help, and jq (the FULL jq language \
                via jaq; -r for raw strings; single-quote filters). Syntax: pipes |, redirects \
                > >> <, sequencing ; and &&. NOT available: shell variables, $(…) substitution, \
                glob expansion in arguments (use find -name or grep -r), cd, loops, ||, sed, \
                awk, xargs, and any network access — GitHub calls go through github_api. Run \
                'help' any time to re-check. Examples: \"cat /r1.json | jq -r '.[] | \
                select(.fork == false) | .full_name' | head -20\", \"grep -in 'error' \
                /r2.json\", \"jq '.items | group_by(.user.login) | map({user: .[0].user.login, \
                n: length})' /r3.json\".",
            "input_schema": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command line to run"
                    }
                },
                "required": ["command"]
            }
        },
        {
            "name": "code_search",
            "description": "Search code on GitHub (the /search/code API) — the fastest way to \
                locate definitions, usages, and examples when exploring unfamiliar code on long \
                tasks, far cheaper than fetching trees and files blindly. The query supports \
                GitHub code-search qualifiers (repo:, org:, user:, path:, filename:, extension:, \
                language:, in:file); pass `repo` to scope to one repository without writing the \
                qualifier yourself. Returns matching files with their matched lines. Limits: \
                searches default branches only, requires an authenticated session, ~10 searches \
                per minute — think first, then make queries specific.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search terms, optionally with qualifiers, e.g. \"LIVE_GEN language:rust\" or \"path:src/ extension:ts useState\""
                    },
                    "repo": {
                        "type": "string",
                        "description": "Scope to one repository as owner/name"
                    },
                    "page": {
                        "type": "integer",
                        "description": "Result page, 1-based (30 files per page)"
                    }
                },
                "required": ["query"]
            }
        },
        {
            "name": "grep",
            "description": "Search file contents in the session's virtual filesystem with a \
                regular expression — saved API responses (/rN.json) and your scratch files. \
                Returns matching lines as path:line:text. The pattern is taken verbatim (no \
                shell quoting needed), so prefer this over bash grep for patterns containing \
                quotes, $ anchors, or backslashes. Searches every file unless `path` narrows \
                it to one file or directory. For structured JSON queries, jq via the bash \
                tool is usually sharper.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regular expression to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "File or directory to search (default: all files)"
                    },
                    "ignore_case": {
                        "type": "boolean",
                        "description": "Case-insensitive matching"
                    }
                },
                "required": ["pattern"]
            }
        },
        {
            "name": "find",
            "description": "List files in the session's virtual filesystem whose name matches \
                a glob (* and ?) — e.g. \"*.json\" for all saved API responses. Returns full \
                paths. Use `path` to limit the search to a directory.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Filename glob, e.g. \"*.json\""
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search under (default /)"
                    }
                },
                "required": ["pattern"]
            }
        }
    ])
}
