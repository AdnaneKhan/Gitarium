# Gitarium

A lightweight GitHub client written entirely in Rust, compiled to WebAssembly, rendered
as a **fully GPU-drawn cyberpunk HUD** in the browser: hand-rolled WebGL (2 or 1, with a Canvas2D software fallback for GPU-less machines)
renderer with SDF rounded rects / borders / neon glows computed in the
fragment shader, a multi-font glyph atlas (Rajdhani for UI, JetBrains Mono
for code) with kerning and letter-tracking, browser-rasterized **color emoji**,
smooth scrolling, hover animations, blinking caret, scanline overlay. No DOM,
no CSS — every pixel comes out of the shader.


Everything is in WASM — layout, widgets, text editing, syntax highlighting, a
from-scratch Markdown renderer, animation timing, GitHub API calls (via
`globalThis.fetch`).

<img width="1323" height="733" alt="image" src="https://github.com/user-attachments/assets/14099f0f-fffa-4b24-9337-9f3aa7ac1ad6" />

<img width="1320" height="660" alt="image" src="https://github.com/user-attachments/assets/0ea6c6a2-11c9-49d3-a3f2-15e3a1f1591c" />

## Quick start — Codespaces (recommended)

The fastest way to run Gitarium, and how it's meant to be used for security work:

1. **Fork** the repo, then **Code → Open with Codespaces** on your fork.
2. The devcontainer installs the toolchain, builds the browser wasm once, and
   **auto-starts the app in proxy mode** in a foreground terminal.
3. Codespaces **forwards port 8080 and opens it** in a browser tab.
4. **Paste a GitHub PAT** at the auth screen (or press Enter for anonymous).

No local toolchain, no build step on your end.

### Why run it this way

- **All GitHub API traffic egresses from the codespace, not your browser.** In
  proxy mode the browser never calls `api.github.com` directly — each request
  rides a WebSocket to the codespace, which performs the `fetch` and returns the
  response. For red-team / engagement work this keeps the API activity off your
  own connection and origin. (AI / agent inference still goes direct.)
- **A scoped PAT is all you need to learn a private repo — no clone required.**
  Grant a PAT read access to the repos you're authorized to assess and browse
  their trees, source, issues, PRs, and CI through the API in seconds. A
  `git clone` of a large repo is slow and noisy; this is fast, lightweight
  reconnaissance, with nothing written to disk.
- **Ambient credentials don't leak.** Proxy mode forwards **only the PAT you
  paste**; the codespace's own auto-injected `GITHUB_TOKEN` is deliberately
  ignored.

### Every GitHub call is logged

In proxy mode each forwarded GitHub request is written to the codespace
terminal as one sanitized line — a built-in audit trail of exactly which
endpoints were hit, and how:

```
2026-06-14T12:34:56.789Z [gitarium-proxy] GET https://api.github.com/repos/acme/widgets/actions/runs?per_page=50
2026-06-14T12:34:56.790Z [gitarium-proxy] DELETE https://api.github.com/repos/acme/widgets/actions/runs/88301234567
2026-06-14T12:34:56.791Z [gitarium-proxy] POST https://api.github.com/repos/acme/widgets/git/blobs {"content":"…","encoding":"base64"}
```

- One line per call, led by an ISO-8601 UTC timestamp: `TIMESTAMP METHOD URL`
  plus the first 100 chars of the request body (if any). The **Authorization
  header is never logged**.
- Tokens are redacted — classic and fine-grained PATs (`ghp_…`, `github_pat_…`)
  and `Bearer …` values become `[REDACTED]`, and newlines are flattened, so no
  entry can span lines or leak a secret.
- The stream is the proxy server's stdout — the same foreground terminal
  `serve.ts --api-proxy` runs in — so it's there live and trivial to redirect
  to a file for a durable audit log.

That record does double duty: it's a **complete audit log of your own API
activity**, and because every call egresses from one codespace origin it's a
faithful picture for **defenders analyzing detection opportunities** — the
endpoint / method / volume pattern they'd actually observe from this kind of
tool. (AI agent inference stays direct and isn't logged here.)

First time in a codespace it asks to allow the auto-serve task (one-time
click). After editing Rust, rebuild with `Ctrl/Cmd+Shift+B` before refreshing.
Full details in [`.devcontainer/README.md`](.devcontainer/README.md).

> **New to the codebase?** [`ARCHITECTURE.md`](docs/ARCHITECTURE.md) is the map —
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
- **AI agent** (`i`) — paste an Anthropic-compatible API key and describe a task in plain
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

## Run locally

Prefer the Codespaces quick start above unless you're developing on this repo.
To serve from your machine:

```sh
bun scripts/serve.ts   # then open http://localhost:8080
```

`serve.ts` negotiates `Content-Encoding` — brotli (≈0.86 MB) or gzip
(≈1.1 MB) — for the wasm/js/html, so the wire transfer is a fraction of the
2.7 MB raw artifact. Any production host should serve the same way (precompressed
`.br`/`.gz`, or on-the-fly).

Paste a PAT at the auth screen (or press Enter for anonymous mode).
Optional: `localStorage.setItem("gitarium_token", "<PAT>")` to skip the prompt.
CORS is a non-issue: `api.github.com` allows cross-origin calls.

### API proxy mode

This is the mode the Codespaces path runs automatically; locally:

```sh
bun scripts/serve.ts --api-proxy   # browser ⇄ server over a WebSocket
```

The browser stops calling `api.github.com` and routes every GitHub request over
a WebSocket (`/__gh`) to the server, which performs the fetch — so GitHub
traffic egresses from the server, not the browser (see Quick start above for
why). The server forwards **only the PAT you paste** — it never reads
`GITHUB_TOKEN`. AI / Anthropic traffic stays direct. When proxying is on, GitHub
calls **hard-fail** if the socket is down rather than silently going direct (the
socket reconnects on the next call).

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
  bun scripts/agent-headless.ts "Triage open issues on owner/repo and label the bugs"
```

It loops autonomously until the model prints a `GOAL_ACHIEVED` /
`GOAL_BLOCKED: <reason>` sentinel (or `AGENT_MAX_TURNS`, default 60, is hit).
Assistant prose goes to stdout, progress to stderr; exit code is 0 on
success. `GITHUB_TOKEN` is optional (anonymous → read-only public access);
`ANTHROPIC_API_KEY` is required, and `ANTHROPIC_BASE_URL` overrides the
Messages API endpoint.

## Single-file build

```sh
bun scripts/build-html.ts              # → dist/gitarium.html (~1.6 MB; wasm embedded gzip'd, gunzipped in-page)
bun scripts/build-html.ts --obfuscate  # …with the wasm run through the obfuscator first
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
`bun scripts/build-html.ts --obfuscate` runs it over the wasm right before embedding
(the last step, so nothing re-optimizes it away). It only raises the
reverse-engineering bar — it is **not** security; a wasm bundle is fully
recoverable. See [`obfuscator/README.md`](obfuscator/README.md).

## Tests

```sh
bun tests/test-browser.ts   # headless-Chrome suite against the live GitHub API
cargo test --workspace      # host-side checks across all crates
```

The suite drives `tests/browser-test.html?mode=suite` and covers auth, org browsing,
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
| `scripts/agent-headless.ts`                 | Headless-agent Bun entrypoint (loads `crates/headless`)  |
| `index.html` / `scripts/serve.ts`           | Browser host (canvas + event glue) / static server       |
| `scripts/build-html.ts` / `tests/test-browser.ts` | Single-file bundler / headless-Chrome suite runner |
| `obfuscator/`                               | Standalone wasm obfuscator (`scripts/build-html.ts --obfuscate`) |
