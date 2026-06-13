# gitarium-obfuscator

A small, **from-scratch** WebAssembly obfuscator for the Gitarium bundle. The
transform logic is ours; only the wasm codec is the canonical
[`walrus`](https://github.com/rustwasm/walrus) IR (hand-rolling a wasm
binary codec would be enormous and far more bug-prone). Written after studying
how existing obfuscators (WasmX, WASMixer, emcc-obf) work — clean-room, no
copied code.

## What it does

Run on a finished `.wasm`, it produces a functionally identical but harder-to-read one.

**On by default (survive a later `wasm-opt`):**

1. **Data-section encryption** (the marquee pass). Every active data segment
   with a constant offset is XOR-scrambled *in the binary*, and a fresh
   function is injected and set as the module `start` so it XOR-decrypts those
   exact memory ranges back **at instantiation, before any export runs**. Any
   pre-existing `start` is chained afterwards, so module init still happens and
   sees plaintext. Net effect: `strings` / grep no longer reveal the API URLs,
   the knowledge bundle, prompts, or error text in the data section (~1.3 MB
   scrambled here), but the app behaves identically. Survives `wasm-opt` (the
   bytes stay scrambled statically; the decrypt loop has memory side effects it
   can't fold away).
2. **Custom-section / metadata stripping**: the `name` and `producers` sections
   are dropped (belt-and-suspenders over `wasm-opt --strip`). This is also the
   single most effective **anti-debug** lever for wasm (see below).

**Opt-in code passes (bloat the binary; MUST be the last step):**

3. `--alias-calls` — route every direct call to a *local* function through a
   fresh funcref table (`call F` → `i32.const slot; call_indirect`), hiding the
   call graph (~24 k calls here).
4. `--obf-consts` — encode every `i32`/`i64` literal as `a ^ (a^n)`, hiding all
   magic numbers / offsets (~125 k constants here).

A `wasm-opt` run *after* these **undoes them** (it folds `a^(a^n)→n` and
devirtualizes `i32.const k; call_indirect → call f`), so when you use them, run
the obfuscator **last** and don't re-optimize. Combined output here: 2.72 → 4.15 MB.

### Why renaming exports is *not* a pass

WasmX renames exported functions; that's **unsafe here** — the JS host resolves
`web_start`, `web_frame`, … by name, so renaming breaks the app unless you also
rewrite the glue. Imports are likewise name-resolved. So we strip *internal*
names only.

### Anti-debug, honestly

wasm anti-debug is far weaker than native, by design: a module **can't read its
own code section** (no self-checksum / self-modifying / patch-detection) and
**can't see engine-level breakpoints**. The real levers are: stripping all
name/DWARF so the DevTools wasm debugger shows only `func[123]` (done here);
code obfuscation so single-stepping is unproductive (`--alias-calls` /
`--obf-consts`); and JS-host-side tricks (timing a `debugger;`, DevTools-open
detection) which live in the glue, not wasm, and are trivially bypassed. All of
it is harassment, not protection.

## Usage

```sh
# after `wasm-pack build --target web`
cargo build --release --manifest-path obfuscator/Cargo.toml

# default (data encryption + strip) — safe to re-pack afterwards:
./obfuscator/target/release/gitarium-obfuscator pkg/gitarium_bg.wasm pkg/gitarium_bg.wasm

# maximum (also code passes) — this must be the LAST step, no wasm-opt after:
./obfuscator/target/release/gitarium-obfuscator --alias-calls --obf-consts \
  pkg/gitarium_bg.wasm pkg/gitarium_bg.wasm
```

Flags: `--no-encrypt`, `--no-strip`, `--alias-calls`, `--obf-consts`.

### Wired into the single-file build

`bun scripts/build-html.ts --obfuscate` runs this tool (all passes) over the wasm
before embedding it — the ideal spot, since `build-html.ts` does no `wasm-opt`
afterward. It writes the obfuscated wasm to a temp file, leaving `pkg/` and the
glue untouched. Verified end-to-end with `bun scripts/build-html.ts --obfuscate --test`
(headless self-test → `SELFTEST-OK`).

## Verified

The obfuscated (and obfuscated-then-`wasm-opt`'d) binary passes the project's
full headless suite — `bun tests/test-browser.ts`: 39/39 + WebGL1/Canvas2D boot
smokes + proxy — proving the decrypt-at-`start` restores memory correctly and
the app is behaviorally unchanged.

## Architecture (to extend)

- `src/main.rs` — CLI + orchestration; parses with `ModuleConfig` (which is how
  the name/producers strip happens), runs passes, re-emits.
- `src/data_xor.rs` — the encryption pass + injected decryptor (a per-segment
  byte loop built with `walrus::FunctionBuilder`).
- `src/code_obf.rs` — the body-rewriting passes (`alias_calls`,
  `obfuscate_constants`) plus a `seq_ids` helper that walks every nested
  instruction sequence in a `LocalFunction`.

A new pass is just a `fn(&mut walrus::Module) -> Result<...>` added to the
orchestration. For code-level passes, iterate `module.funcs` (the
`LocalFunction` instruction sequences) — keep stack effects identical and
re-validate against the suite.

## Caveat

Obfuscation ≠ security. A wasm bundle is fully downloadable and decompilable;
the XOR key ships in the binary, so the data is recoverable by anyone who runs
or reads the decryptor. This only raises the reverse-engineering bar — it does
**not** protect a secret. Don't ship credentials in the bundle.
