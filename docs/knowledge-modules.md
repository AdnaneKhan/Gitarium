# Knowledge Modules

Knowledge modules give the in-app agent pre-baked reference material —
domain knowledge it can consult instead of relying on training data.
They follow the Claude Skills file shape (a `SKILL.md` with frontmatter
plus reference files) but are **read-only knowledge, not executable
skills**: modules are selected at build time, compiled into the wasm
binary, and mounted into the agent's in-memory VFS where it already has
`bash`, `grep`, and `find` tools to navigate them.

The motivating example is a condensed GitHub REST v3 reference so that
agents — especially on smaller models — can compose correct, complex
API calls instead of guessing endpoint schemas.

## Repository layout

```
knowledge/                  # all modules live here, one dir each
  github-api/
    SKILL.md                # frontmatter + table of contents (≤100 lines)
    references/
      checks.md             # one topic per file (≤200 lines each)
      pulls.md
      ...
knowledge.toml              # build-time selection
```

`knowledge.toml` is minimal — a list of module names to compile into
this build:

```toml
modules = ["github-api"]
```

## Frontmatter

```yaml
---
name: github-api            # must match the directory name
description: Condensed GitHub REST v3 reference — endpoints, schemas,
  pagination rules. Consult before composing non-trivial API calls.
source: https://docs.github.com/rest
fetched: 2026-06-12
---
```

`name` and `description` are required; `source` and `fetched` are
required for modules condensed from external docs (knowledge goes
stale — the date lets the agent hedge and lets us audit). The
`description` drives the agent's decision to consult the module, so it
must say *when* to look, not just what's inside. Cap: 300 chars.

## Build process

A `build.rs` step reads `knowledge.toml`, validates each selected
module, deflate-compresses the module tree into one bundle embedded via
`include_bytes!`, and the app inflates it into the VFS at startup
(`miniz_oxide` both sides; `src/knowledge.rs` owns the runtime half).

The build **fails** on: missing/oversized frontmatter fields, `name` ≠
directory name, any non-markdown file in a module, `SKILL.md` over 100
lines, a reference file over 200 lines, or a module over 1 MB
uncompressed (warning at 256 KB — binary size is paid by every user on
page load).

Markdown-only is deliberate. Claude-Skills-style executable scripts are
excluded: a script file in the VFS tempts the model to execute it, and
the mini-shell would half-run it confusingly. Runnable know-how ships
instead as fenced one-liner commands inside the markdown, written
against the in-app shell's real command set (pipes, redirects, `jq`) —
e.g. a recipe for extracting failing check runs from a stored
`/rN.json` response.

## VFS mount

Modules mount at `/knowledge/<name>/…`, a **reserved read-only
prefix**: `vfs::write`, `append`, and `remove` refuse paths under it,
and `vfs::clear()` (the CLEAR chip / new session) wipes scratch files
only, re-seeding `/knowledge/` so it survives every session reset.
This also keeps `ls /` output clean — scratch responses at the root,
reference material under one prefix.

## Agent surfacing

`agent::system_prompt()` appends a static block listing each compiled
module:

```
Knowledge modules (read-only, under /knowledge/):
  github-api — Condensed GitHub REST v3 reference … Consult before
  composing non-trivial API calls.
Before composing a request you are not certain about, grep the relevant
module (e.g. grep -i "check runs" /knowledge/github-api/) and read the
matching reference file instead of guessing.
```

Listing alone is not enough for small models — the explicit
consult-first instruction is part of the contract. The block is static
per build, so it stays inside the cached prompt prefix.

The intended access pattern is grep-then-read-one-file: `SKILL.md` is a
table of contents, topics are split into small reference files, and the
agent should never need to `cat` a whole module.

## Non-goals (v1)

- Executable scripts of any kind.
- Fetching modules at runtime as separate assets (revisit if the
  embedded bundle pushes binary size too far).
- Per-user or runtime module toggles — selection is fixed at build.
