# RustVM Architecture

RustVM is a GitHub client written almost entirely in Rust, compiled to
WebAssembly, and drawn as a GPU-rendered HUD on a single `<canvas>` — no DOM,
no CSS. The JS host is a thin pipe: canvas events in, frames out. The same
core also powers a headless, self-driving Claude agent that operates GitHub
with no UI at all.

This document orients a new contributor (human or AI) to the codebase: where
things live, how data flows, and the non-obvious invariants you must respect
when editing. For day-to-day conventions (the **200-line file cap**, build/test
commands) see `CLAUDE.md`; for user-facing features see `README.md`.

---

## Orientation: start here

**Mental model.** One `App` struct is the entire UI state machine (routes,
messages, all input handling) and holds *only* logic — no rendering, no IO.
Each frame, a `View` walks `App` state and emits an immediate-mode `DrawList`,
which a backend (WebGL2/WebGL1/Canvas2D) rasterizes. All IO (GitHub, Claude) is
async: a handler spawns a future that resolves to a `Msg`, which re-enters the
synchronous `App` at the next frame. Nothing blocks; nothing mutates state
mid-event.

**Where to change X:**

| You want to… | Look in |
| --- | --- |
| add/alter a keybinding or screen behavior | `crates/app/src/app/keys.rs`, `code_keys.rs` |
| add a new async data flow (request → state) | a handler in `crates/app/src/app/*.rs` + a `Msg` variant in `msg.rs` |
| add a GitHub REST call | `crates/core/src/github/` |
| change how a screen looks | `crates/render/src/px/view/<screen>_pane.rs` |
| add a modal/overlay | `Overlay` in `app/state.rs` + `app/overlays.rs` + `px/view/overlay_*.rs` |
| change editor behavior | `crates/app/src/app/editor/` |
| add a syntax-highlighted language | `crates/ui/src/highlight/langs.rs` |
| change the agent's tools or loop | `crates/agent/src/agent/` (client) + `app/agent_loop.rs`, `agent_history.rs` (driver) |
| touch the JS↔wasm boundary | `src/lib.rs`, `src/web_input.rs`, `index.html` |
| change build size / distribution | `Cargo.toml`, `build-html.ts`, `serve.ts` |

**Before editing, read [Cross-cutting invariants](#cross-cutting-invariants).**
The async staleness guards, the Messages-API history discipline, and the
"per-frame hit regions are stale" rule are subtle and easy to break.

---

## Workspace & build targets

A Cargo workspace (`members = ["crates/*"]`, resolver 2). Pure logic lives in
rlib crates; two thin **cdylib** crates are the wasm entrypoints and bundle only
what each target needs.

| Crate | Type | Functionality |
| --- | --- | --- |
| `rustvm-core` | rlib | VFS, `fetch`, GitHub REST, the in-wasm shell, knowledge bundle |
| `rustvm-ui` | rlib | input model (`Key`/`Mods`/`Event`), grid, theme, syntax highlighting |
| `rustvm-agent` | rlib | Claude client (Messages API, tools) + the headless driver |
| `rustvm-app` | rlib | the `App` state machine + async runtime → agent, core, ui |
| `rustvm-render` | rlib | the `px` WebGL/Canvas2D renderer → app, core, ui |
| `rustvm` (root) | **cdylib** | **web target**: app + render → the `web_*` exports |
| `rustvm-headless` | **cdylib** | **headless target**: agent only → `agent_run_headless` |

The headless target depends *only* on `rustvm-agent`, so it links no
renderer/UI — half the size of the web bundle.

```sh
cargo check --target wasm32-unknown-unknown   # primary dev check
cargo test --workspace                         # native unit tests
wasm-pack build --target web                   # web → pkg/
wasm-pack build crates/headless --target web   # headless → crates/headless/pkg/
```

`[profile.release]` is tuned for wasm size: `opt-level="z"`, `lto="fat"`,
`codegen-units=1`, `panic="abort"`, `strip=true`, then a wasm-pack-managed
`wasm-opt -Oz` pass (the `--enable-*` flags are load-bearing — without them
wasm-opt rejects modern rustc's wasm features and silently skips the size pass).

---

## The runtime loop (how a frame happens)

The browser host (`index.html`, mirrored in `build-html.ts`) imports the
`web_*` exports from `src/lib.rs` / `src/web_input.rs` and drives an
**event → frame → message** cycle.

**Host exports** (`src/lib.rs`, `src/web_input.rs`):
- `web_start(canvas_id, font_px, token)` — seeds the knowledge bundle, builds
  `Renderer`/`View`/`App` into a thread-local `HOST`, renders frame 0.
- `web_frame(t_ms)` — the requestAnimationFrame tick: **drains queued messages
  first**, then redraws only if dirty/animating.
- input: `web_key` (returns `true` ⇒ host should `preventDefault`),
  `web_mouse_down/up/move`, `web_wheel`, `web_paste`, `web_context_menu`,
  `web_cursor_style` (polled CSS cursor), `web_set_font_px` (DPR change).
- `web_debug_text()` — text dump of the last frame; the test harness's only
  observation channel.

**The async cycle.** All network/agent work goes through `spawn_msg`
(`crates/app/src/lib.rs`): `spawn_local` runs the future, pushes its `Msg`
onto a thread-local queue, then calls `host_wake()` — a JS function the host
installs on `globalThis` that schedules a `web_frame` (rAF is paused in hidden
tabs, so a result landing off-loop must explicitly request a frame). The next
`web_frame` calls `drain_msgs`, popping every queued `Msg` into `App::on_msg`.

```
event/click ─▶ handler ─▶ spawn_msg(async { …github/anthropic…; Msg::X{…} })
                                              │ (off the event loop)
                            MSGS queue ◀──────┘ + host_wake()
 next web_frame: drain_msgs ─▶ App::on_msg(Msg) ─▶ on_x() ─▶ mutate state, dirty
                 render ─▶ View::frame builds DrawList ─▶ backend rasterizes
```

State mutation happens only inside `on_msg` handlers and event handlers — never
inside a spawned future, which can only *produce a `Msg`*.

---

## Subsystems

### The App state machine (`crates/app/src/app/`)

The `App` is routes/screens, all keyboard and mouse handling, and the async
data lifecycle. Pure logic, split into per-topic `impl App` blocks across many
≤200-line modules.

| Path | Role |
| --- | --- |
| `mod.rs` | the `App` struct + fields, `App::new`, re-exports, `fmt_age` |
| `state.rs` | shared types: `Route`, `RepoSource`, `Overlay`, `ConfirmAction`, `Click`, `Scroll`, `Layout`, `OpenFile`, `SearchScope`, `Loadable<T>` |
| `msg.rs` | the `Msg` enum (one variant per async request) + `on_msg` dispatch |
| `keys.rs` | `on_event` (top-level dispatch), `repos_key`, the `plain(mods)` gate |
| `code_keys.rs` | Repo-route keys (`repo_key`→`code_key`/`viewer_key`) |
| `auth.rs` / `input.rs` | token entry; `on_paste` + `perform_click` |
| `repos.rs` | streamed paginated listing (`repos_gen`), filter, sort |
| `repo.rs` / `repo_msgs.rs` | `RepoView`, `open_repo`, branch/tree handlers |
| `files.rs` / `file_msgs.rs` | open/commit a file + their staleness guards |
| `tree.rs`, `staging.rs`, `commit.rs`, `menu.rs` | tree nav, staged workspace, multi-file commit, right-click menu |
| `actions.rs` | Actions tab (runs/jobs) |
| `search.rs` / `code_search.rs` | find-file palette; GitHub code search (`code_search_gen`) |
| `overlays.rs` | overlay key dispatch + the simple modals |

**Routes.** `Route` (Auth → Repos → Repo / Agent) is the top-level screen. The
open repo lives in `App.rv: Option<RepoView>` (branch, `Loadable` collections,
flattened tree `rows`, the open `file`, staged workspace, focus). One
`App.overlay: Option<Overlay>` holds any modal; one `App.context_menu` the
floating right-click menu. Async data uses `Loadable<T>` =
Idle/Loading/Ready/Failed rather than scattered flags.

**Keyboard/click dispatch.** `on_event` is the single entry; **every handler
returns `bool` = "consumed"**, which the host uses to decide `preventDefault`
(unconsumed keys keep native browser behavior). Order: a key first dismisses
any context menu; then if an overlay is open it goes to `overlay_key`; else it
dispatches by route. `plain(mods)` = `!ctrl && !alt`; char bindings are gated
on it so single-letter shortcuts never shadow browser/OS shortcuts (the host
maps Cmd→ctrl, so `Cmd+R` falls through unconsumed). Clicks arrive
*already hit-tested* by the renderer as a `Click` enum and are applied by
`perform_click`; clicking the already-selected item re-dispatches `Enter`
(click-to-select, click-again-to-activate).

**Overlays.** `Overlay` is the modal system (`Commit`, `BranchPick`,
`OpenRepo`, `NewFile`, `NewBranch`, `FileSearch`, `CodeSearch`, `Help`,
`Confirm`). `Confirm { action: ConfirmAction }` is the unsaved-edits gate: any
navigation that would discard a modified buffer routes through it.

### The renderer (`crates/render/src/px/`)

A pure **view** over `App`. Each frame `View::frame` walks state and emits a
`DrawList`; a backend rasterizes it. **The View owns only geometry, animation,
and hit-regions; `App` owns all state.**

| Group | File | Role |
| --- | --- | --- |
| draw/ | `draw/mod.rs` (`DrawList`, `RectF`, `MODE_*`), `draw/text.rs` | immediate-mode API + the interleaved vertex stream + CPU clip stack; glyph quads & text measurement |
| render/ | `render/mod.rs` (`Renderer`/`Backend`), `webgl.rs`, `shaders.rs`, `canvas2d.rs`, `glyphs.rs` | backend acquisition + the three backends |
| px/ | `atlas.rs`, `anim.rs` (`Smooth`), `theme.rs` | glyph atlas, easing, palette |
| view/ | `mod.rs` (`View`), `frame.rs`, `input.rs`, per-screen `*_pane.rs`, `overlay_*.rs`, `widgets.rs` | the view layer |

**The draw pipeline — one lossless quad stream.** Every primitive becomes
exactly one quad (6 verts), and **every vertex carries the primitive's full
parameters**: `pos2 uv2 color4 rect4 param4` (16 floats), where `rect` =
(center, half-extent) and `param` = `(radius, feather, border_width, MODE_*)`.
So the stream decodes losslessly back into primitives. `solid`/`rrect`/
`border`/`glow`/`scanlines`/`text` just set the mode. Clipping is CPU-side
(axis-aligned rect intersection stack).

**Backends.** `Renderer::new` acquires **webgl2 → webgl → canvas2d**. The WebGL
path drives a WebGL2 *or* WebGL1 context through the WebGL1 bindings (every call
exists on both), with GLSL ES 1.00, a LUMINANCE atlas, and no VAOs. The
fragment shader branches on `param.w`: glyph samples `.r` coverage, SDF computes
`sdBox` with a `smoothstep` falloff (inner subtraction for borders). **The
shared-stream insight:** machines with no WebGL fall back to `canvas2d.rs`,
which *interprets the exact same vertex stream* (solid→`fillRect`,
SDF→rounded path / shadow-blur, glyph→batched blit). There is no second scene
representation — the quad stream is the single source of truth for all three.

**Font atlas** (`atlas.rs`): three embedded TTFs (Rajdhani Regular/Bold,
JetBrains Mono), rasterized on demand via `fontdue` into one shelf-packed
texture; cache key `(font, size_bucket, char)`, missing glyphs fall back across
fonts then to `'?'`.

**Hit-testing → Click/Scroll.** Screen modules *record* hit regions as they
draw (`self.clicks.push((RectF, Click))`, `self.wheels.push(...)`). At event
time `click_at`/`wheel` scan those vecs **in reverse** so the topmost-drawn
wins; clicks become `app.perform_click`, wheels write the row index back into
`App` so keyboard nav stays coherent. The editor and agent transcript use
cached geometry (`editor_geom`, `agent_geom`) for pixel→(row,col) math instead
of discrete rects.

### Editor, input & syntax highlighting

The editable code-viewing stack — all pure logic, no rendering.

| Path | Role |
| --- | --- |
| `crates/app/src/app/editor/{mod,undo,keys}.rs` | the `Editor` buffer + undo/redo + key handling |
| `crates/ui/src/ui/{input,lineinput,grid,theme}.rs` | `Key`/`Mods`/`Event`, single-line `LineInput`, geometry, palette |
| `crates/ui/src/highlight/{mod,langs}.rs` | the syntax highlighter |

**Editor model.** `lines: Vec<String>` (newline-free; trailing newline tracked
out-of-band). `Pos = (row, col)` where **`col` is a char index**. Selection is
`anchor: Option<Pos>` + `cursor`; any mutation clears the anchor (selection
never survives an edit). `selection_text` is defensively bounds-checked so a
stale selection degrades to no-copy, never a panic.

**Undo is involutive + grouped** (`undo.rs`): an `Op` *is* an inverse edit, and
`apply(op)` mutates the buffer **and returns the inverse `Op`**, so undo and
redo share one path. `Op::Group(Vec<Op>)` is one atomic step: a
replace-selection emits `Group([restore_deleted, delete_new])`, so a single
Ctrl+Z restores the whole replace. Consecutive single-char inserts coalesce
into one undo step; a replace group does not coalesce with following typing.

**Tab/column duality.** Tabs occupy `TAB_W` visual cells but one char column, so
two coordinate spaces exist: `col_to_x` (col→visual cell, for cursor/scroll)
and `x_to_col` (cell→col, for hit-testing). The view converts a click pixel to
a cell x, then calls `x_to_col`. Mixing the two spaces silently corrupts edits
on tab-indented files.

**Input primitives.** `Key`/`Mods`/`Event` (`ui/input.rs`) are the *only* input
vocabulary the app matches on — `Event::Key(Key, Mods)` and `Event::Paste`.
Mouse/wheel are **not** here; the view handles those in pixel space.
`LineInput` is the single-line model used by every overlay/auth/commit field
(independent of `Editor` — no selection/undo).

**Syntax highlighting** is line-anchored with one cross-line bit of state.
`highlight(spec, line, entry_state) -> (Vec<Span>, exit_state)` lexes one line;
`Span = (start, end, Rgb)` in char indices. The only state crossing lines is
`LineState { Normal, InBlockComment }`. `LangSpec` (line/block comments, string
delims, keywords, md flag) is `static`; `lang_for_path` maps extension →
`&'static LangSpec`. Each `OpenFile` caches `line_states` (entry state per line)
which `rehighlight` folds top-to-bottom; `line_states.len()` must always equal
`lines.len()`.

### Core services (`crates/core/`)

The runtime substrate every target shares: GitHub REST, an in-memory FS, a
from-scratch shell over it, HTTP, and a compiled knowledge bundle.

| Path | Role |
| --- | --- |
| `github/{mod,types,repos,content,gitdb,actions,search}.rs` | the REST client |
| `fetch.rs` | `globalThis.fetch` binding + rate-limit tracking |
| `vfs.rs` | the in-memory filesystem (`/rN.json`, scratch, read-only `/knowledge/`) |
| `sh/*.rs` | the bash interpreter (`run`, parse, words, exec, command groups, `jq`) |
| `knowledge.rs` + `build.rs` | compile/deflate `knowledge/` into the binary |

**GitHub client.** Everything funnels through `api`/`api_with_accept` (sets
`Accept` + API version, `Bearer` only when a token is present) → `parse`
(checks 2xx, surfaces GitHub's `{message}`). **Every call takes
`&Option<String>` token** — `None` = anonymous (public, 60/hr); there is no
global auth state, the caller threads it, and the client never caches.
Pagination is **streamed**: `repos_first_page`/`repos_page` fetch one page; the
UI chains the next off each result (`MAX_PAGES` guards runaways). A 404 on the
orgs endpoint is disambiguated via one `/users/{owner}` probe (user → retry as
a user; org → access error; missing → not-found) so a private org never
silently reads as a short public list. **Code search is dual**: `search_code`
(GUI, one highlighted line per file) vs `search_code_global` (agent, up to 3
lines + an `incomplete` flag). `gitdb.rs` composes blob→tree→commit→ref for
atomic multi-file commits (vs `content.rs:put_file`'s one-file-one-commit).

**In-wasm shell & VFS** — *why it exists*: GitHub responses are large, so
instead of flooding the agent's context they're saved as `/rN.json` files
(`vfs::store_response`) and navigated with a real bash-like shell (in the spirit
of vercel-labs' just-bash; **no OS/network access**). `sh::run` handles
`;`/`&&`/pipes/redirects and a fixed command set (`ls cat head tail grep wc sort
uniq cut base64 find echo rm mkdir touch jq pwd`); removed commands return
*teaching* errors (`curl`→"use the github_api tool"). **jq is the full language
via jaq**, not a subset. Output is capped at 8 000 chars but data is never lost
(it stays in the VFS for a narrower follow-up). The VFS is a thread-local
`BTreeMap`; `/knowledge/` is a read-only mount that survives `clear`.

**Knowledge bundle**: markdown reference docs validated and deflated into the
binary by `build.rs`, inflated into `/knowledge/` at startup; `prompt_block`
lists them in the system prompt so the agent greps them before composing
uncertain requests.

### The Claude agent

An autonomous GitHub agent (model `claude-opus-4-8`) speaking the raw Anthropic
**Messages API** over `globalThis.fetch`. It ships in two forms sharing *all*
logic except the driver: the in-app `i` window and a headless self-driving
binary.

| Path | Role |
| --- | --- |
| `crates/agent/src/agent/mod.rs` | system prompt, `build_request`, `complete` (the POST), key/url persistence |
| `…/tools.rs`, `calls.rs`, `exec.rs` | tool schemas; parse `tool_use`→`ToolCall`; execute (+ VFS spill) |
| `…/compact.rs`, `headless.rs` | token accounting/overflow; the self-driving loop |
| `crates/app/src/app/chat.rs` | `AgentChat` state (transcript, verbatim `history`, `gen`, `pending`, …) |
| `…/agent_loop.rs` | `agent_turn`, `on_agent_response` (stop_reason dispatch), tool batch |
| `…/agent_history.rs` | `LIVE_GEN`, `push_user_text`, `sanitize_history_tail`, cancel/clear |
| `…/agent_compact.rs` | proactive + reactive context compaction |

**The loop.** A user message bumps `gen`, sets `busy`, and fires `agent_turn`
(`spawn_msg` → `Msg::AgentResponse`). `on_agent_response` appends the assistant
content and switches on `stop_reason`:

```
user msg ─▶ agent_turn ─▶ complete() ─▶ Msg::AgentResponse ─▶ on_agent_response
   ├ tool_use   → exec() each call (cancel-checked) → history += tool_results → loop
   ├ pause_turn → resend (capped at 8)
   ├ refusal / max_tokens / end_turn → busy=false; sanitize_history_tail
```

**Tools.** `github_api` (one REST call; responses > `INLINE_LIMIT` 2 000 chars
spill to `/rN.json` and the model gets a path + shape summary, not the bytes),
`code_search`, and `bash`/`grep`/`find` over the in-wasm shell. Every tool
result is clipped (~24 000 chars) before entering the conversation — recoverable
because the full data is in the VFS.

**History is a verbatim invariant, not a log.** `AgentChat.history` is the exact
Messages array (assistant blocks echoed back so thinking↔tool_use pairing stays
valid), and the API 400s on malformed shapes, so:
- **`sanitize_history_tail`** strips unanswered `tool_use` blocks and drops
  empty/thinking-only trailing turns. **Every terminal path funnels through it**
  (end_turn, refusal, max_tokens, failed pause_turn resend, cancel), so the next
  send never hits a dangling tool_use or empty turn.
- empty content arrays (pre-output refusals) are never pushed.
- new user text **merges into** a trailing user (tool_result) turn rather than
  appending a second user message (the API rejects consecutive user turns).

**Cancellation = `gen` + `LIVE_GEN`.** `gen` (bumped on send/cancel/clear) is
captured by each future; stale responses whose `gen` no longer matches are
dropped. But a tool *batch already mid-execution* (sequential, some calls
mutating) is the hard case: `LIVE_GEN` is a thread-local mirror the detached
future re-checks **between executions** and breaks on, so an in-flight cancel
stops the *remaining* mutating calls instead of letting them outlive the cancel.

**Compaction.** Token usage accumulates into `ctx_tokens`; when it crosses
`SOFT_CAP` (proactive) or `complete()` returns an overflow error (reactive), the
loop diverts to summarize the history into a single user turn (`tool_choice:none`,
same system/tools so the cached prefix is reused). **History is replaced only on
success** — every failure path leaves a valid sendable history; the shell VFS
survives compaction.

**Headless mode** (`headless.rs` → `agent_run_headless`, driven by
`agent-headless.ts`): the same loop, self-driving toward a goal, ending when the
model prints a leading-line sentinel `GOAL_ACHIEVED` / `GOAL_BLOCKED: <reason>`
(or the turn cap → `max_turns`). Progress streams as JSON events; exit 0 only on
success.

---

## Cross-cutting invariants

These cut across subsystems and are the easiest things to break. Read them
before editing.

1. **Async results carry full discriminators; stale ones are dropped.** Every
   `Msg` that mutates state carries enough to detect staleness, and its handler
   verifies before mutating:
   - **Generation counters** — `repos_gen` (repo listing), `code_search_gen`
     (code search), and the agent's `gen` are bumped on each (re)issue; handlers
     drop results whose gen no longer matches.
   - **Identity discriminators** — repo handlers compare `repo`/`branch`/`run_id`
     against the live `RepoView`. `on_file_loaded` checks repo **and branch**
     (a stale old-branch file would attach the wrong base sha and corrupt the
     next commit). `on_committed` always toasts but only mutates if still on the
     same repo.
   - **Cross-repo open** rides `RepoView.pending_open_path` (not `App`), so a
     superseding open discards the pending jump along with the whole view — it
     can never target the wrong repo.

2. **The Messages-API history is an invariant, not a log.** Every path that ends
   an agent turn must leave `history` in a shape the API accepts (alternation,
   tool_use answered, non-empty content). Centralized in
   `sanitize_history_tail`; never hand-edit the tail without going through it.

3. **Cancellation must cover side effects, not just results.** Bumping `gen`
   orphans the reply; the sequential tool batch must re-check `LIVE_GEN` between
   steps or mutating calls outlive the cancel.

4. **Per-frame hit regions/geometry are stale at event time.** `clicks`,
   `wheels`, `editor_geom`, `agent_geom`, `menu_rects` describe the *last drawn*
   frame; events fire between frames. Index through them **defensively**
   (`agent_geom?`, `.min(len-1)`, `saturating_sub`) — in wasm, one bad index
   panics and kills the whole app. Opening an overlay clears the hit vecs so it
   swallows main-screen input.

5. **One quad stream, three backends.** The renderer emits a single lossless
   vertex stream that WebGL2, WebGL1, and Canvas2D all consume. Don't add a
   second scene representation; new primitives must encode into the
   `pos/uv/color/rect/param` vertex layout so every backend can decode them.

6. **Hit-test with the same metrics you draw with.** Selection/click resolution
   on text must use the exact font/size/tracking the line was drawn with (see
   `agent_xs` / `atlas.char_xs`); an assumed-uniform advance silently
   mis-targets on bold, tracked, or non-ASCII lines.

7. **The 200-line file cap** (`CLAUDE.md`): every *source* file ≤200 lines; split
   into focused modules rather than grow one (parents hold shared state, siblings
   export `pub(super)`). Docs like this file are exempt.

8. **Vendored `jaq-std`** (`vendor/jaq-std`, via `[patch.crates-io]`): a verbatim
   3.0.1 fork that drops jiff's bundled IANA tzdb (dead weight on wasm; jq time
   builtins are UTC-only). Feature unification is additive so it can't be
   disabled from our manifests — re-apply the one-line jiff edit when bumping
   (see `vendor/jaq-std/VENDOR.md`).

---

## Build, test & distribution quick reference

**Distribution** (three forms from the same web wasm):
- **Served `pkg/`** — `bun serve.ts` negotiates brotli/gzip `Content-Encoding`
  (the ~2.7 MB wasm ships ~0.86 MB brotli), decoded transparently by the browser.
- **Single-file** — `bun build-html.ts` → `dist/rustvm.html` (~1.5 MB): glue
  inlined, wasm embedded as **gzip** base64, self-decompressed in-page via
  `DecompressionStream` (gzip not brotli — browsers have no native JS brotli
  decoder; brotli is used only on the wire).
- **Headless CLI** — `bun agent-headless.ts "<goal>"` (env: `ANTHROPIC_API_KEY`
  required, `GITHUB_TOKEN`/`ANTHROPIC_BASE_URL`/`AGENT_MAX_TURNS` optional).

**Testing:**
- `cargo test --workspace` — native unit tests.
- `bun test-browser.ts` — headless-Chrome suite against the live GitHub API,
  scraping `PASS/FAIL/SUITE:` from the console. It runs the full suite on WebGL2,
  then API-free boot smokes forcing WebGL1 (`?gl=1`) and Canvas2D (`?gl=0`). The
  page observes state only through `web_debug_text()`/`document.title`. Token
  comes from `$GITHUB_TOKEN` or a gitignored `.env.test`.
