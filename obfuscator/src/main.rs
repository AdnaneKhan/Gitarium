//! A small, from-scratch WebAssembly obfuscator for the Gitarium bundle.
//!
//! Clean-room: the *techniques* (data-section encryption with an injected
//! startup decryptor, custom-section stripping) are our own; only the wasm
//! codec is the canonical `walrus` IR. It transforms a finished `.wasm` and
//! re-emits a valid one — verify the output with the project's browser suite.
//!
//! Note: obfuscation ≠ security. A wasm bundle is fully recoverable; this only
//! raises the reverse-engineering bar (e.g. `strings` no longer reveals the
//! API URLs / knowledge bundle / prompts that live in the data section).

use anyhow::{bail, Result};
use clap::Parser;

mod code_obf;
mod data_xor;

#[derive(Parser)]
#[command(about = "Obfuscate a Gitarium .wasm (data encryption + section stripping)")]
struct Args {
    /// Input .wasm
    input: String,
    /// Output .wasm
    output: String,
    /// Disable data-section encryption.
    #[arg(long)]
    no_encrypt: bool,
    /// Disable custom-section / name stripping.
    #[arg(long)]
    no_strip: bool,
    /// Route direct calls to local functions through a funcref table
    /// (`call_indirect`), hiding the call graph. Must be the last build step.
    #[arg(long)]
    alias_calls: bool,
    /// Encode every i32/i64 literal as `a ^ (a^n)` (bloats; must be the last
    /// build step — a later wasm-opt would fold it away).
    #[arg(long)]
    obf_consts: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let bytes = std::fs::read(&args.input)?;

    // Parsing with these off means walrus won't re-emit the name/producers
    // sections — a free strip of the most readable metadata.
    let mut cfg = walrus::ModuleConfig::new();
    if !args.no_strip {
        cfg.generate_name_section(false);
        cfg.generate_producers_section(false);
    }
    let mut module = cfg.parse(&bytes)?;

    if !args.no_strip {
        let ids: Vec<_> = module.customs.iter().map(|(id, _)| id).collect();
        let n = ids.len();
        for id in ids {
            module.customs.delete(id);
        }
        eprintln!("strip: removed {} custom section(s)", n);
    }

    if !args.no_encrypt {
        let report = data_xor::encrypt_data(&mut module)?;
        eprintln!(
            "encrypt: {} segment(s), {} bytes scrambled (key 0x{:02x})",
            report.segments, report.bytes, report.key
        );
        if report.segments == 0 {
            bail!("no encryptable data segments found — aborting (would be a no-op)");
        }
    }

    if args.alias_calls {
        let n = code_obf::alias_calls(&mut module);
        eprintln!("alias-calls: routed {} direct call(s) through a table", n);
    }

    if args.obf_consts {
        let n = code_obf::obfuscate_constants(&mut module);
        eprintln!("obf-consts: encoded {} literal constant(s)", n);
    }

    let out = module.emit_wasm();
    std::fs::write(&args.output, &out)?;
    eprintln!("wrote {} ({} bytes)", args.output, out.len());
    Ok(())
}
