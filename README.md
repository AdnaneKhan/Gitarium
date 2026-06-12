# RustVM

A GitHub client written entirely in Rust, compiled to WebAssembly, rendered
as a **fully GPU-drawn cyberpunk HUD** in the browser: hand-rolled WebGL2
renderer with SDF rounded rects / borders / neon glows computed in the
fragment shader, a multi-font glyph atlas (Rajdhani for UI, JetBrains Mono
for code) with kerning and letter-tracking, smooth scrolling, hover
animations, blinking caret, scanline overlay. No DOM, no CSS — every pixel
comes out of the shader.

Rust does everything — layout, widgets, text editing, syntax highlighting,
animation timing, GitHub API calls (via `globalThis.fetch`). The JS host is
a thin pipe (~70 lines): canvas events in, frames out.

> The earlier terminal (TUI) mode was removed to keep the codebase lean; if
> server-side returns it will likely be a headless background agent, not a UI.

## Features

- PAT auth (fine-grained recommended) or anonymous mode for public repos
- Repository list with filtering; open any `owner/repo` directly, or enter a
  bare organization/user name to browse all of its repositories
- File tree browsing, branch switching
- File viewer/editor: syntax highlighting (Rust/JS/TS/Py/Go/C/JSON/TOML/YAML/MD),
  selection, undo/redo, auto-indent
- Single-file commits to any branch via the Contents API
- GitHub Actions: workflow runs, jobs, and step status
- AI agent window (`i`): paste an Anthropic API key and describe a task in
  plain language; Claude drives the GitHub REST API autonomously through a
  generic tool — list/triage issues, open PRs, inspect CI, anything the
  API allows under your PAT. Key is stored in localStorage next to the PAT.
  Large API responses land as files in an in-wasm mini-shell (pipes,
  redirects, grep, full jq via jaq) that the agent navigates instead of
  flooding its context — in the spirit of vercel-labs/just-bash.
- Full mouse support (hover, click, wheel) and complete keyboard parity;
  `?` shows the keymap

## Build

```sh
rustup target add wasm32-unknown-unknown   # once
wasm-pack build --target web               # outputs ./pkg (~3 MB, ~1.2 MB gzipped)
```

## Run

```sh
bun serve.ts   # then open http://localhost:8080
```

Paste a PAT at the auth screen (or press Enter for anonymous mode).
Optional: `localStorage.setItem("rustvm_token", "<PAT>")` to skip the prompt.
CORS is a non-issue: `api.github.com` allows cross-origin calls.

## Single-file build

```sh
bun build-html.ts   # → dist/rustvm.html (~2 MB)
```

One self-contained HTML file — glue inlined, wasm + fonts embedded as
base64. No server, no other files: double-click it (works from `file://`)
or drop it on any static host.

## Tests

```sh
bun test-browser.ts   # headless-Chrome suite against the live GitHub API
cargo test            # host-side checks (font glyph coverage)
```

The suite drives `browser-test.html?mode=suite` and covers auth, org
browsing, the user-account fallback, repo/tree/file flows, editing, the
commit dialog, undo, the branch picker, and Actions runs/jobs. The same
page also has screenshot drive modes (`?repo=…`, `?mode=actions`,
`?mode=branches`, `?mode=org`).

## Layout

| Path               | Purpose                                                        |
| ------------------ | -------------------------------------------------------------- |
| `src/app/`         | State machine (routes, messages, keymap) + text editor         |
| `src/px/`          | GPU UI: SDF WebGL2 renderer, multi-font atlas, HUD views       |
| `src/ui/`          | Shared primitives: colors, rects, key events, line-input model |
| `src/github.rs`    | GitHub REST client (repos/orgs/trees/contents/commits/actions) |
| `src/agent.rs`     | Claude agent: Anthropic Messages API + generic GitHub tool     |
| `src/sh.rs`        | Agent's in-memory mini-shell (pipes, grep, jq via jaq)         |
| `src/vfs.rs`       | Virtual filesystem backing the shell (API responses, notes)    |
| `src/fetch.rs`     | `globalThis.fetch` binding                                     |
| `src/highlight.rs` | Hand-rolled per-language lexers                                |
| `index.html`       | Browser host (canvas + event glue)                             |
| `serve.ts`         | Static server                                                  |
| `build-html.ts`    | Single-file bundler                                            |
| `test-browser.ts`  | Headless-Chrome suite runner                                   |

The legacy `fetch_url()` export from the project's first iteration still
works.
