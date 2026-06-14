# Devcontainer image for Gitarium. Based on the official Rust dev image and
# adds the three pieces it lacks: the wasm32 target, wasm-pack, and Bun.
#
# Everything runs as the non-root `vscode` user so tools land in the right
# home and the system-wide cargo dir (writable by this user in the base image)
# keeps installs on PATH for shells, the integrated terminal, and tasks alike.
FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

USER vscode

# wasm32 target — both the web and headless crates compile to it.
RUN rustup target add wasm32-unknown-unknown

# wasm-pack (same installer CI uses) → $CARGO_HOME/bin, already on PATH.
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Bun: serve.ts / build-html.ts / the browser test harness run TypeScript
# directly, so there is no package.json and no install step.
RUN curl -fsSL https://bun.sh/install | bash
ENV BUN_INSTALL=/home/vscode/.bun
ENV PATH=/home/vscode/.bun/bin:${PATH}
