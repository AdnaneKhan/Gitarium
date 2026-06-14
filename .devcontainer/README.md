# Gitarium dev container

Opens the repo in a GitHub Codespace (or any dev-container host) with the full
toolchain and the browser demo running in **proxy mode**, so every GitHub API
request is routed through the codespace instead of straight from the browser.

## What it sets up

- Rust (stable) + the `wasm32-unknown-unknown` target
- `wasm-pack`
- `bun`
- Builds the browser wasm (`pkg/`) once on create
- Forwards port **8080** and opens it in a browser tab
- Auto-starts `bun scripts/serve.ts --api-proxy` in a dedicated **foreground
  terminal** — **but only inside a GitHub Codespace**, via a `runOn: folderOpen`
  task (`.vscode/tasks.json` → `auto-serve-codespace`, calling
  `.devcontainer/auto-serve.sh`). Local VS Code, with or without a devcontainer,
  stays silent

## How proxy mode works here

1. `serve.ts --api-proxy` serves the static assets **and** upgrades `/__gh` to a
   WebSocket GitHub proxy (`scripts/proxy-server.ts`).
2. Served HTML is injected with `window.__GITARIUM_PROXY__ = "/__gh"`; the page
   builds the WebSocket URL from `location.host`, so it always points at the
   codespace's forwarded URL — nothing is hardcoded.
3. The wasm `proxy` module sends each GitHub request over that socket; the
   codespace performs the actual `fetch` to `api.github.com` and returns the
   response. Anthropic/agent traffic stays direct (never proxied).

Net effect: **all GitHub API traffic egresses from the codespace.**

## Tokens

GitHub auth in proxy mode uses **only the PAT you paste at the auth screen**:
the server forwards it per-request and never reads `GITHUB_TOKEN`. The
codespace's own auto-injected `GITHUB_TOKEN` is deliberately ignored, so no
ambient credentials leak into the session. Paste a PAT for writes (commit /
merge), or press Enter for anonymous (read-only).

`ANTHROPIC_API_KEY` is an optional Codespace secret (set it at
<https://github.com/settings/codespaces>); only needed for the in-app (`i`) or
headless agent.

## Running it

**In a Codespace** the server auto-starts in its own **foreground terminal** when
the workspace opens (the `auto-serve-codespace` task, proxy mode) — logs stream
there live and Ctrl+C stops it. The first time, VS Code/Codespaces asks you to
allow the automatic task (one-time). Port 8080 forwards and opens automatically.

**Locally** nothing auto-starts — serve manually:

- `bun scripts/serve.ts --api-proxy` — proxy mode
- `bun scripts/serve.ts` — direct mode (browser calls GitHub itself)

After editing Rust, rebuild before refreshing the page:

- `Ctrl/Cmd+Shift+B` runs the default `build-web` task (`wasm-pack build --target web`).

## Other useful commands

```sh
cargo check --target wasm32-unknown-unknown   # primary dev check
cargo test --workspace                         # native unit tests
wasm-pack build crates/headless --target web   # headless-agent wasm
bun scripts/agent-headless.ts "<goal>"         # headless agent (needs ANTHROPIC_API_KEY)
```
