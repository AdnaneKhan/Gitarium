# Gitarium

A lightweight GitHub client written entirely in Rust, compiled to WebAssembly, rendered
as a **fully GPU-drawn cyberpunk HUD** in the browser: hand-rolled WebGL (2 or 1, with a Canvas2D software fallback for GPU-less machines)
renderer with SDF rounded rects / borders / neon glows computed in the
fragment shader, a multi-font glyph atlas (Rajdhani for UI, JetBrains Mono
for code) with kerning and letter-tracking, browser-rasterized **color emoji**,
smooth scrolling, hover animations, blinking caret, scanline overlay. No DOM,
no CSS — every pixel comes out of the shader.

Rust does everything — layout, widgets, text editing, syntax highlighting, a
from-scratch Markdown renderer, animation timing, GitHub API calls (via
`globalThis.fetch`).

> **New to the codebase?** [`ARCHITECTURE.md`](ARCHITECTURE.md) is the map —
> how the crates fit together, the runtime loop, each subsystem, and the
> cross-cutting invariants to respect when editing.

## Features

- **Auth** — fine-grained PAT (recommended) or anonymous mode for public repos
- **Repositories** — filterable list (sort, hide forks/archived); open any
  `owner/repo` directly, or a bare org/user name to browse all of its repos
- **Code** — file-tree browsing; branch switching and branch creation
  (`b` → `+ New` / `n`: pick a base, name it, Create); a viewer/editor with
  syntax highlighting (Rust/JS/TS/Py/Go/C/JSON/TOML/YAML/MD), selection,
  undo/redo, auto-indent; find-file palette (`/`) and GitHub code search (`g`)
- **Staged commits** — add / edit / delete files locally (`s` stage, `n` new,
  `d` delete, `u` unstage; the `+ FILE` button; or a tree right-click menu that
  shows staged adds/deletes inline), then commit them as **one atomic commit**
  via the Git Database API (blobs → tree → commit → ref), with optional
  author / committer / date overrides and a destination chip in the commit
  dialog (current branch, a new branch, or a new tag)
- **Issues & Pull Requests** — the 100 most-recently-updated open issues / PRs
  per tab (loaded lazily), with **colored labels**. Open one to read its body
  and comments rendered as **Markdown** — headings, lists (incl. task lists),
  tables, blockquotes, fenced code (syntax-highlighted), inline
  bold/italic/`code`/strikethrough, links — plus **color emoji** and GitHub
  `:shortcode:` emoji. Mouse text-selection + copy, and in-page search (`/`),
  in the detail. For PRs: mergeability, CI checks and reviews at a glance, and
  **approve / merge** (`a` / `m`, with a merge/squash/rebase method chip)
- **Actions** — workflow runs, their jobs and step status, and drill into a
  job's **raw logs** with in-page search (`/`), text-selection + copy, and
  download
- **Download** — right-click a folder (or the repo root) to download it as a
  `.tar.gz`, built **in-wasm** from the current branch's blobs
- **AI agent** (`i`) — paste an Anthropic API key and describe a task in plain
  language; Claude drives the GitHub REST API autonomously through a generic
  tool — list/triage issues, open PRs, inspect CI, anything the API allows under
  your PAT. Large responses land as files in an in-wasm mini-shell (pipes,
  redirects, grep, full jq via jaq) the agent navigates instead of flooding its
  context — in the spirit of vercel-labs/just-bash. Key stored in localStorage.
- Full mouse support (hover, click, drag-select, wheel) and keyboard parity.
  Four repo tabs — **Code · Issues · Pulls · Actions** (`t`/`p`/`a` switch,
  Esc backs out); `?` shows the full keymap.

## Build

The code is a Cargo workspace of functional crates (`crates/`), layered so
each wasm target bundles only what it needs:

| Crate           | Functionality                                              |
| --------------- | ---------------------------------------------------------- |
| `gitarium-core`   | VFS, `fetch`, GitHub REST, the in-wasm shell, knowledge    |
| `gitarium-agent`  | Claude agent loop + tools + the headless driver → core     |
| `gitarium-ui`     | input model, grid, theme, syntax highlighting              |
| `gitarium-app`    | UI state machine + async runtime → agent, core, ui         |
| `gitarium-render` | WebGL/Canvas2D GPU renderer, font atlas, Markdown → app, … |
| `gitarium` (root) | **web target** cdylib: app + render → `web_*` exports      |
| `gitarium-headless` | **headless target** cdylib: agent only → `agent_run_headless` |

```sh
rustup target add wasm32-unknown-unknown      # once
wasm-pack build --target web                  # web target → ./pkg (~2.7 MB, ~1.1 MB gzip)
wasm-pack build crates/headless --target web  # headless agent → ~1.2 MB
cargo test --workspace                        # host-side checks
```

## Run

```sh
bun serve.ts   # then open http://localhost:8080
```

`serve.ts` negotiates `Content-Encoding` — brotli (≈0.86 MB) or gzip
(≈1.1 MB) — for the wasm/js/html, so the wire transfer is a fraction of the
2.7 MB raw artifact. Any production host should serve the same way (precompressed
`.br`/`.gz`, or on-the-fly).

Paste a PAT at the auth screen (or press Enter for anonymous mode).
Optional: `localStorage.setItem("gitarium_token", "<PAT>")` to skip the prompt.
CORS is a non-issue: `api.github.com` allows cross-origin calls.

### API proxy (optional)

```sh
bun serve.ts --api-proxy                      # browser ⇄ server over a WebSocket
GITHUB_TOKEN=ghp_… bun serve.ts --api-proxy   # …with a server-held token
```

With `--api-proxy` the browser stops calling `api.github.com` directly: every
GitHub request is forwarded to the server over a WebSocket (`/__gh`), which
performs the fetch and forwards the response back. AI/Anthropic inference still
goes **directly** from the browser. Token model — *support both*: if
`GITHUB_TOKEN` is set the server uses it (the browser needs no PAT — press Enter
at the auth screen to log in with the server's identity); otherwise the browser's
forwarded token is used. When proxying is on, GitHub calls **hard-fail** if the
socket is down rather than silently going direct (the socket reconnects on the
next call).

## Headless agent

The same agent that powers the in-app `i` window also runs detached and
UI-free — same tools (`github_api`, `code_search`, `bash`/`grep`/`find` over
the in-wasm shell), the compiled knowledge bundle, the shell VFS, and context
compaction — but self-driving toward a goal instead of chatting.

It builds as its own minimal wasm (the `gitarium-headless` crate — agent +
foundation, no renderer, ~1.2 MB vs ~2.7 MB for the web bundle):

```sh
wasm-pack build crates/headless --target web   # once
GITHUB_TOKEN=ghp_…  ANTHROPIC_API_KEY=sk-ant-… \
  bun agent-headless.ts "Triage open issues on owner/repo and label the bugs"
```

It loops autonomously until the model prints a `GOAL_ACHIEVED` /
`GOAL_BLOCKED: <reason>` sentinel (or `AGENT_MAX_TURNS`, default 60, is hit).
Assistant prose goes to stdout, progress to stderr; exit code is 0 on
success. `GITHUB_TOKEN` is optional (anonymous → read-only public access);
`ANTHROPIC_API_KEY` is required, and `ANTHROPIC_BASE_URL` overrides the
Messages API endpoint.

## Single-file build

```sh
bun build-html.ts              # → dist/gitarium.html (~1.6 MB; wasm embedded gzip'd, gunzipped in-page)
bun build-html.ts --obfuscate  # …with the wasm run through the obfuscator first
```

One self-contained HTML file — glue inlined, wasm + fonts embedded as
base64. No server, no other files: double-click it (works from `file://`)
or drop it on any static host.

## Obfuscation (optional)

`obfuscator/` is a small **from-scratch** wasm obfuscator (built on the
canonical `walrus` IR). Passes: **data-section encryption** with an injected
`start` decryptor (so `strings` no longer reveals the API URLs / knowledge
bundle / prompts), custom-section stripping, and opt-in code passes —
direct→`call_indirect` **call-graph aliasing** and **literal encoding**.
`bun build-html.ts --obfuscate` runs it over the wasm right before embedding
(the last step, so nothing re-optimizes it away). It only raises the
reverse-engineering bar — it is **not** security; a wasm bundle is fully
recoverable. See [`obfuscator/README.md`](obfuscator/README.md).

## Tests

```sh
bun test-browser.ts      # headless-Chrome suite against the live GitHub API
cargo test --workspace   # host-side checks across all crates
```

The suite drives `browser-test.html?mode=suite` and covers auth, org browsing,
the user-account fallback, repo/tree/file flows, editing, the commit dialog,
undo, the branch picker, Actions runs/jobs, and the Issues/Pulls lists and
detail (body + comments, PR merge requirements, text-selection + copy, in-page
search). The same page has screenshot drive modes (`?repo=…`, `?mode=actions`,
`?mode=branches`, `?mode=org`, `?mode=emoji`, `?mode=search`).

## Layout

See the crate table under [Build](#build) for the functional split. Within
the crates:

| Path                                | Purpose                                                  |
| ----------------------------------- | -------------------------------------------------------- |
| `crates/app/src/app/`               | State machine (routes, messages, keymap) + text editor   |
| `crates/render/src/px/`             | SDF WebGL renderer + Canvas2D fallback, font atlas, views |
| `crates/render/src/px/view/md/`     | From-scratch Markdown parser + layout (issue/PR content) |
| `crates/render/src/px/emoji.rs`     | Color-emoji rasterization (OS emoji font → color atlas)  |
| `crates/ui/src/ui/`, `…/highlight/` | Input model, grid, theme; per-language lexers            |
| `crates/core/src/github/`           | GitHub REST client (repos/orgs/trees/contents/actions/issues/pulls/checks) |
| `crates/agent/src/agent/`           | Claude agent (Messages API, tools) + `headless` driver   |
| `crates/core/src/sh/`, `vfs.rs`     | In-memory mini-shell (pipes, grep, jq) + its VFS         |
| `crates/app/src/app/download.rs`    | Folder/repo → in-wasm `.tar.gz` archive                  |
| `crates/core/src/fetch.rs`          | `globalThis.fetch` binding                               |
| `src/lib.rs`, `src/web_input.rs`    | Root web cdylib: `Host` + `web_*` exports                |
| `agent-headless.ts`                 | Headless-agent Bun entrypoint (loads `crates/headless`)  |
| `index.html` / `serve.ts`           | Browser host (canvas + event glue) / static server       |
| `build-html.ts` / `test-browser.ts` | Single-file bundler / headless-Chrome suite runner       |
| `obfuscator/`                       | Standalone wasm obfuscator (`build-html.ts --obfuscate`) |
