#!/usr/bin/env bash
# Auto-start the Gitarium demo in proxy mode — ONLY inside a GitHub Codespace,
# in the foreground of a dedicated VS Code terminal.
#
# Invoked by the `auto-serve-codespace` task in .vscode/tasks.json with
# `runOptions.runOn: folderOpen`, so the server opens in its own terminal pane
# whenever the codespace opens. Running in a real integrated terminal (rather
# than a backgrounded postStartCommand child) is what keeps it alive: the
# terminal system owns the process, so it isn't reaped when the lifecycle hook
# returns. The $CODESPACES guard makes this a no-op anywhere else.
#
# For a non-Codespace serve, or to drop the proxy, run the `serve-proxy` /
# `serve-direct` tasks (or `bun scripts/serve.ts [--api-proxy]`) directly.
set -euo pipefail

cd "$(dirname "$0")/.."

if [ "${CODESPACES:-}" != "true" ]; then
  echo "gitarium: not a GitHub Codespace — skipping auto-serve."
  echo "          to serve manually: bun scripts/serve.ts --api-proxy"
  exit 0
fi

# postCreateCommand builds the wasm; this task runs after, on paper — but wait
# for the artifact in case they overlap, so the page doesn't 404 mid-build.
WASM=pkg/gitarium_bg.wasm
if [ ! -f "$WASM" ]; then
  echo "gitarium: waiting for the wasm build to finish before serving…"
  waited=0
  until [ -f "$WASM" ] || [ "$waited" -ge 900 ]; do
    sleep 10; waited=$((waited + 10))
  done
  if [ ! -f "$WASM" ]; then
    echo "gitarium: wasm still missing after ~15 min — serving anyway; the page will 404 until 'wasm-pack build --target web' is run." >&2
  fi
fi

echo "gitarium: serving 'serve.ts --api-proxy' in the foreground (port 8080)."
echo "          logs stream here; Ctrl+C to stop."
exec bun scripts/serve.ts --api-proxy
