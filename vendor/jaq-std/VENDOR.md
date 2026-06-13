# Vendored: jaq-std 3.0.1 (slim-jiff fork)

A verbatim copy of `jaq-std` 3.0.1 from crates.io with **one change**: its
`jiff` dependency is declared `default-features = false, features = ["std",
"tz-system"]` (see `Cargo.toml`). Wired in via `[patch.crates-io]` in the
workspace-root `Cargo.toml`.

## Why
`jaq-json` depends on `jaq-std` with default features, which enables
`jaq-std`'s `time` feature → `jiff` with *default* features → the bundled
IANA timezone database (`tz-fat`, `tzdb-concatenated`, `tzdb-bundle-platform`,
`tzdb-zoneinfo`). That database is ~260 KB of dead weight in the wasm:
jaq's jq time builtins (`now`, `gmtime`, `strftime`, `todate`/`fromdate`)
are UTC/system-based and never resolve named zones, and in a browser
`TimeZone::system()` resolves to UTC whether or not the db is bundled.
Feature unification is additive, so this can't be turned off from our own
manifests — hence the fork.

Measured saving on the web target: raw −261 KB, gzip −92 KB.

## Updating
When bumping jaq-std:
1. `cp -r ~/.cargo/registry/src/index.crates.io-*/jaq-std-<ver>/* vendor/jaq-std/`
2. Re-apply the `[dependencies.jiff]` change above (and drop `[[test]]` /
   `[dev-dependencies]`).
3. Bump the version in the patch requirement if the major changes.
4. `wasm-pack build --target web` + `cargo test --workspace` to verify.
