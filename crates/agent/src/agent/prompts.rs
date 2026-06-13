//! Runtime access to the agent's prompt text. The strings live deflated in
//! prompts.bin (built from build_prompts.rs by build.rs) and are inflated once
//! on first use, so they never appear in plaintext in the shipped wasm. Edit
//! the text in build_prompts.rs; callers look it up here by key.

use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/prompts_meta.rs")); // pub static KEYS: &[&str]

static BUNDLE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/prompts.bin"));
static TEXT: OnceLock<Vec<String>> = OnceLock::new();

/// Inflate the bundle once into per-key strings, in KEYS order.
fn text() -> &'static [String] {
    TEXT.get_or_init(|| {
        let raw = miniz_oxide::inflate::decompress_to_vec(BUNDLE).expect("prompts bundle corrupt");
        let mut out = Vec::with_capacity(KEYS.len());
        let mut i = 0;
        while i < raw.len() {
            let n = u32::from_le_bytes(raw[i..i + 4].try_into().unwrap()) as usize;
            let end = i + 4 + n;
            out.push(String::from_utf8(raw[i + 4..end].to_vec()).expect("prompts not UTF-8"));
            i = end;
        }
        out
    })
}

/// Prompt text for `key` (defined in build_prompts.rs). Keys are compile-time
/// literals at every call site, so an unknown key is a build-time typo.
pub(super) fn get(key: &str) -> &'static str {
    let idx = KEYS
        .iter()
        .position(|k| *k == key)
        .unwrap_or_else(|| panic!("unknown prompt key: {key}"));
    text()[idx].as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_inflates_nonempty() {
        for key in KEYS {
            assert!(!get(key).is_empty(), "empty prompt for {key}");
        }
    }

    #[test]
    fn known_markers_present() {
        assert!(get("system").contains("autonomous GitHub operations agent"));
        assert!(get("tool_bash").contains("There is no real OS"));
        assert!(get("compact_instruction").contains("handoff"));
    }
}
