# RustVM — conventions

## File size cap

Source files are capped at **200 lines**, and the whole tree currently
complies. When a change would push a file past the cap, split it into
focused modules instead of growing it; never let a new file start life
over the cap. Module layout: parents hold shared state/types (children
may use parent privates), siblings export `pub(super)`.

## Build & test

- `cargo check --target wasm32-unknown-unknown` — primary target
- `cargo test` — native unit tests (app, sh, github, px)
- `wasm-pack build --target web && bun test-browser.ts` — headless-Chrome
  suite against the live GitHub API
