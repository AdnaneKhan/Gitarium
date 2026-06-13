//! Knowledge modules: reference material compiled into the binary by
//! build.rs and inflated into the read-only /knowledge/ VFS mount at
//! startup (design: docs/knowledge-modules.md).

use crate::vfs;

// `MODULES: &[(&str, &str)]` — (name, description) per compiled module.
include!(concat!(env!("OUT_DIR"), "/knowledge_meta.rs"));

static BUNDLE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/knowledge.bin"));

/// Inflate the bundle into the VFS. Call once at startup; clear() never
/// drops the mount, so there is nothing to re-seed later.
pub fn seed() {
    if MODULES.is_empty() {
        return;
    }
    let raw = miniz_oxide::inflate::decompress_to_vec(BUNDLE).expect("knowledge bundle corrupt");
    let mut i = 0;
    while i < raw.len() {
        let (path, next) = record(&raw, i);
        let (content, next) = record(&raw, next);
        vfs::seed(path, content.to_string());
        i = next;
    }
}

/// One length-prefixed UTF-8 record (u32 LE length, then bytes).
fn record(raw: &[u8], i: usize) -> (&str, usize) {
    let n = u32::from_le_bytes(raw[i..i + 4].try_into().unwrap()) as usize;
    let end = i + 4 + n;
    (std::str::from_utf8(&raw[i + 4..end]).expect("knowledge bundle not UTF-8"), end)
}

/// System-prompt block listing compiled modules; empty when none.
pub fn prompt_block() -> String {
    if MODULES.is_empty() {
        return String::new();
    }
    let mut s = String::from("\nKnowledge modules — read-only reference files under /knowledge/:");
    for (name, desc) in MODULES {
        s.push_str(&format!("\n  {} — {}", name, desc));
    }
    s.push_str(
        "\nBefore composing a request you are not certain about, search the \
         relevant module (e.g. grep -r -i \"check runs\" /knowledge/github-api/) \
         and read the matching reference file instead of guessing.",
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sh;

    const SKILL: &str = "/knowledge/github-api/SKILL.md";

    #[test]
    fn seeds_bundle_into_vfs() {
        seed();
        let skill = vfs::read(SKILL).expect("SKILL.md seeded");
        assert!(skill.contains("name: github-api"));
        assert!(vfs::read("/knowledge/github-api/references/conventions.md").is_some());
    }

    #[test]
    fn knowledge_mount_is_read_only() {
        seed();
        let before = vfs::read(SKILL).unwrap();
        assert!(!vfs::write(SKILL, "x".into()));
        assert!(!vfs::append(SKILL, "x"));
        assert!(!vfs::remove(SKILL));
        assert!(!vfs::write("/knowledge/new.md", "x".into()));
        assert_eq!(vfs::read(SKILL).unwrap(), before);

        let (msg, ok) = sh::run(&format!("rm {}", SKILL));
        assert!(!ok && msg.contains("read-only"), "rm: {}", msg);
        let (msg, ok) = sh::run("echo x > /knowledge/scratch.md");
        assert!(!ok && msg.contains("read-only"), "redirect: {}", msg);
        let (_, ok) = sh::run(&format!("cat {}", SKILL));
        assert!(ok, "reads must still work");
    }

    #[test]
    fn clear_keeps_knowledge_drops_scratch() {
        seed();
        vfs::write("/scratch.txt", "x".into());
        vfs::clear();
        assert!(vfs::exists(SKILL));
        assert!(!vfs::exists("/scratch.txt"));
    }

    #[test]
    fn prompt_block_lists_modules() {
        let block = prompt_block();
        assert!(block.contains("github-api"));
        assert!(block.contains("/knowledge/"));
    }
}
