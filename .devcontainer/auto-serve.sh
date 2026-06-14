#!/usr/bin/env bash
# Auto-start the Gitarium demo in proxy mode — ONLY inside a GitHub Codespace.
#
# Invoked from the devcontainer `postStartCommand`. That alone restricts it to
# devcontainer environments (plain local VS Code never runs postStart), and the
# `$CODESPACES` guard below additionally skips local devcontainers — so the
# server starts exclusively in a Codespace. It is backgrounded because
# postStartCommand must return; logs stream to /tmp/gitarium-serve.log and its
# PID is recorded so the manual `serve-proxy` task can replace it cleanly.
#
# For an interactive foreground serve (live logs, easy restart) run the
# `serve-proxy` task, or: bun scripts/serve.ts --api-proxy
set -euo pipefail

LOG=/tmp/gitarium-serve.log
PIDFILE=/tmp/gitarium-serve.pid

if [ "${CODESPACES:-}" != "true" ]; then
  echo "gitarium: not a GitHub Codespace — skipping auto-serve."
  echo "          to serve manually: bun scripts/serve.ts --api-proxy"
  exit 0
fi

# postStartCommand can re-run (e.g. on rebuild); don't stack a second server.
if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then
  echo "gitarium: background serve already running (pid $(cat "$PIDFILE")) — leaving it."
  exit 0
fi

echo "gitarium: Codespace detected — starting 'serve.ts --api-proxy' in background (port 8080)."
echo "          logs:    tail -f $LOG"
echo "          replace: run the 'serve-proxy' task"
# Detach so the server survives the postStartCommand shell exiting — plain
# `nohup &` can get reaped when the lifecycle command returns. setsid (Linux
# util-linux, present in the codespace) puts it in its own session, the reliable
# fix; nohup+disown is the fallback for hosts without setsid (e.g. macOS). Either
# way the long-lived bun PID lands in the PID file for the manual task to stop.
if command -v setsid >/dev/null 2>&1; then
  setsid bash -c 'echo $$ > /tmp/gitarium-serve.pid; exec bun scripts/serve.ts --api-proxy' \
    >"$LOG" 2>&1 </dev/null &
else
  nohup bun scripts/serve.ts --api-proxy >"$LOG" 2>&1 </dev/null &
  echo $! > /tmp/gitarium-serve.pid
fi
disown 2>/dev/null || true
