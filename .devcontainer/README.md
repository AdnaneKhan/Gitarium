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
- Auto-starts `bun scripts/serve.ts --api-proxy` in the background — **but only
  inside a GitHub Codespace**, gated on the `$CODESPACES` env var (see
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

Declared as Codespace secrets (the devcontainer prompts for them on first
create); set them at <https://github.com/settings/codespaces>.

| Secret | Used by | Notes |
| --- | --- | --- |
| `GITHUB_TOKEN` | the proxy (server-side) | When set, the proxy **overrides** the in-browser token for every GitHub call. If unset, the codespace's own `GITHUB_TOKEN` (if injected) or whatever you type in the app is forwarded as-is. The codespace token's scopes may be limited — use a PAT for writes (commit / merge). |
| `ANTHROPIC_API_KEY` | the agent | Optional; only needed to use the in-app (`i`) or headless agent. |

## Running it

**In a Codespace** the server auto-starts in the background on container start
(proxy mode). Tail its logs with `tail -f /tmp/gitarium-serve.log`, or run the
`serve-proxy` task for an interactive foreground serve (it replaces the
background one). Port 8080 forwards and opens automatically.

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
