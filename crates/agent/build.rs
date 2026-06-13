//! Compress the in-app agent prompt text (build_prompts.rs) into a deflate
//! bundle the wasm inflates at runtime (src/agent/prompts.rs). Mirrors
//! crates/core/build.rs: this keeps the agent's system prompt, tool
//! descriptions, and compaction strings out of plaintext in the shipped wasm
//! (they no longer show up in `strings gitarium_bg.wasm`).

use std::env;
use std::fs;
use std::path::PathBuf;

include!("build_prompts.rs"); // pub const PROMPTS: &[(&str, &str)]

fn main() {
    println!("cargo:rerun-if-changed=build_prompts.rs");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Length-prefixed records (u32 LE len, then UTF-8 bytes) in PROMPTS order;
    // KEYS carries the lookup names so the bundle holds only the text.
    let mut raw = Vec::new();
    let mut keys = String::new();
    for (key, text) in PROMPTS {
        raw.extend_from_slice(&(text.len() as u32).to_le_bytes());
        raw.extend_from_slice(text.as_bytes());
        keys.push_str(&format!("{:?}, ", key));
    }
    let packed = miniz_oxide::deflate::compress_to_vec(&raw, 8);
    fs::write(out.join("prompts.bin"), packed).unwrap();
    fs::write(
        out.join("prompts_meta.rs"),
        format!("pub static KEYS: &[&str] = &[{}];\n", keys),
    )
    .unwrap();
}
