#!/usr/bin/env bash
# Hardened release build of the browser wasm.
#
# On top of the [profile.release] floor in Cargo.toml (strip, panic=abort, -Oz,
# fat LTO), this rebuilds std with panic_immediate_abort so panics become a bare
# trap — which deletes nearly every embedded panic-message and source-path
# string (file!()/Location::caller) from the binary — and remaps + drops any
# residual location detail (--remap-path-prefix + -Z location-detail=none).
# Source-path sanitization lives HERE, not in the manifest: the `trim-paths`
# profile key is still nightly-gated on the pinned cargo and would break stable
# `cargo`/`wasm-pack` builds. Net effect: the source-tree map and dependency
# paths that `strings gitarium_bg.wasm` used to dump are gone, and the binary is
# a touch smaller.
#
# Needs the nightly toolchain + rust-src; the script installs both if either is
# missing (so it self-bootstraps on a host that only has stable Rust — e.g. a
# fresh codespace). Stable `wasm-pack build --target web` still works for dev;
# use this for anything you actually ship.
set -euo pipefail

cd "$(dirname "$0")/.."

TOOLCHAIN="${GITARIUM_NIGHTLY:-nightly}"

# Self-bootstrap: install the toolchain if missing (and rust-src alongside it)
# so this works on a host that only has stable Rust, such as a fresh codespace.
if ! rustup run "$TOOLCHAIN" rustc --version >/dev/null 2>&1; then
  echo "• installing $TOOLCHAIN toolchain (needed for the hardened std rebuild)…"
  rustup toolchain install "$TOOLCHAIN" --profile minimal --component rust-src
fi
if ! rustup component list --toolchain "$TOOLCHAIN" --installed 2>/dev/null | grep -q '^rust-src'; then
  echo "• adding rust-src to $TOOLCHAIN (needed to rebuild std)…"
  rustup component add rust-src --toolchain "$TOOLCHAIN"
fi

# Remap the three build roots that embed the local username (and machine
# layout) into any residual path string: the project tree, the cargo registry,
# and the rustup toolchain (std src, since build-std recompiles it). These three
# don't overlap, so prefix-match precedence is moot. panic_immediate_abort
# removes most such strings outright; this sanitizes whatever survives.
REMAP="--remap-path-prefix=$PWD=/build"
REMAP="$REMAP --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo"
REMAP="$REMAP --remap-path-prefix=$HOME/.rustup=/rust"

echo "• building hardened wasm with $TOOLCHAIN (immediate-abort panics + rebuilt std)…"
# Passed via env, not `wasm-pack -- -Z …` (wasm-pack mis-parses a leading `-Z`
# after `--` as the crate path):
#   • CARGO_UNSTABLE_BUILD_STD = std,panic_abort — recompile std/core so the
#       panic strategy can be swapped in.
#   • RUSTFLAGS:
#       -Cpanic=immediate-abort (+ -Zunstable-options to accept it) makes panics
#         a bare trap → panic-message and source-path strings stop being emitted.
#         Set here rather than via the profile because the profile form
#         (panic="immediate-abort") needs a manifest cargo-features gate that
#         would break stable `cargo check`/`test`. RUSTFLAGS is appended after
#         the profile's -Cpanic=abort, so immediate-abort wins.
#       -Zlocation-detail=none drops any residual file/line; $REMAP rewrites the
#         build roots (see above).
RUSTUP_TOOLCHAIN="$TOOLCHAIN" \
CARGO_UNSTABLE_BUILD_STD="std,panic_abort" \
RUSTFLAGS="${RUSTFLAGS:-} -Zunstable-options -Cpanic=immediate-abort -Z location-detail=none $REMAP" \
  wasm-pack build --target web --release

WASM=pkg/gitarium_bg.wasm
echo "• built $WASM ($(wc -c < "$WASM" | tr -d ' ') bytes)"

# Self-check: fail loudly if the things this build is meant to remove are still
# in the clear. Covers both the leaked source paths and the agent prompt IP
# (the latter is compressed by crates/agent/build.rs — see prompts.rs).
leak=0
check() { # <label> <needle>
  if strings -n 6 "$WASM" | grep -qiF -- "$2"; then
    echo "  ✗ LEAK: $1 still present in plaintext ('$2')" >&2
    leak=1
  else
    echo "  ✓ $1 absent"
  fi
}
check "absolute build paths" "$HOME"
check "source module paths"  "crates/agent/src"
check "agent system prompt"  "autonomous GitHub operations agent"
check "bash tool prompt"     "There is no real OS"

if [ "$leak" -ne 0 ]; then
  echo "error: plaintext that should have been stripped/compressed is still present" >&2
  exit 1
fi

echo "• ok — no tracked plaintext leaks"
if command -v bun >/dev/null 2>&1; then
  echo "• regenerating dist/gitarium.html…"
  bun scripts/build-html.ts
  echo "• done → dist/gitarium.html"
else
  echo "• skip dist bundle (bun not found); run: bun scripts/build-html.ts"
fi
