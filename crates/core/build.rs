//! Compile knowledge modules (knowledge.toml + knowledge/) into a
//! deflate-compressed bundle the app inflates into the read-only
//! /knowledge/ VFS mount at startup. Enforces the budgets from
//! docs/knowledge-modules.md — violations fail the build.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const SKILL_CAP: usize = 100; // SKILL.md max lines
const REF_CAP: usize = 200; // reference file max lines
const DESC_CAP: usize = 300; // frontmatter description max chars
const WARN_BYTES: usize = 256 << 10;
const FAIL_BYTES: usize = 1 << 20;

fn main() {
    println!("cargo:rerun-if-changed=knowledge.toml");
    println!("cargo:rerun-if-changed=knowledge");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut records: Vec<(String, String)> = Vec::new();
    let mut meta = String::new();
    for name in module_list() {
        let desc = pack_module(&name, &mut records);
        meta.push_str(&format!("({:?}, {:?}), ", name, desc));
    }

    let mut raw = Vec::new();
    for (path, content) in &records {
        raw.extend_from_slice(&(path.len() as u32).to_le_bytes());
        raw.extend_from_slice(path.as_bytes());
        raw.extend_from_slice(&(content.len() as u32).to_le_bytes());
        raw.extend_from_slice(content.as_bytes());
    }
    let packed = miniz_oxide::deflate::compress_to_vec(&raw, 8);
    fs::write(out.join("knowledge.bin"), packed).unwrap();
    fs::write(
        out.join("knowledge_meta.rs"),
        format!("pub static MODULES: &[(&str, &str)] = &[{}];\n", meta),
    )
    .unwrap();
}

/// Module names from knowledge.toml; no file means no modules.
fn module_list() -> Vec<String> {
    let Ok(text) = fs::read_to_string("knowledge.toml") else {
        return Vec::new();
    };
    let inner = text
        .split("modules")
        .nth(1)
        .and_then(|s| s.split('[').nth(1))
        .and_then(|s| s.split(']').next())
        .unwrap_or_else(|| panic!("knowledge.toml: expected modules = [\"name\", …]"));
    inner.split('"').skip(1).step_by(2).map(str::to_string).collect()
}

/// Validate one module and append its files to `records` as
/// (/knowledge/<name>/…, content) pairs. Returns the description.
fn pack_module(name: &str, records: &mut Vec<(String, String)>) -> String {
    let dir = Path::new("knowledge").join(name);
    let mut files = Vec::new();
    walk(&dir, &mut files);
    files.sort();

    let skill = fs::read_to_string(dir.join("SKILL.md"))
        .unwrap_or_else(|_| panic!("{}: missing SKILL.md", name));
    if skill.lines().count() > SKILL_CAP {
        panic!("{}: SKILL.md over {} lines", name, SKILL_CAP);
    }
    let fm = frontmatter(name, &skill);
    let get = |k: &str| {
        fm.iter()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| panic!("{}: frontmatter missing '{}'", name, k))
    };
    if get("name") != name {
        panic!("{}: frontmatter name does not match directory", name);
    }
    let desc = get("description");
    if desc.is_empty() || desc.chars().count() > DESC_CAP {
        panic!("{}: description empty or over {} chars", name, DESC_CAP);
    }

    let mut total = 0;
    for f in &files {
        if f.extension().and_then(|e| e.to_str()) != Some("md") {
            panic!("{}: non-markdown file {} (modules are md-only)", name, f.display());
        }
        let content = fs::read_to_string(f)
            .unwrap_or_else(|_| panic!("{}: unreadable (non-UTF-8?) {}", name, f.display()));
        if f.file_name().and_then(|n| n.to_str()) != Some("SKILL.md")
            && content.lines().count() > REF_CAP
        {
            panic!("{}: {} over {} lines", name, f.display(), REF_CAP);
        }
        total += content.len();
        let rel: Vec<&str> = f
            .strip_prefix("knowledge")
            .unwrap()
            .components()
            .map(|c| c.as_os_str().to_str().unwrap())
            .collect();
        records.push((format!("/knowledge/{}", rel.join("/")), content));
    }
    if total > FAIL_BYTES {
        panic!("{}: {} KB uncompressed, over the 1 MB cap", name, total >> 10);
    }
    if total > WARN_BYTES {
        println!("cargo:warning={}: {} KB uncompressed (soft cap 256 KB)", name, total >> 10);
    }
    desc
}

fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
    println!("cargo:rerun-if-changed={}", dir.display());
    let entries =
        fs::read_dir(dir).unwrap_or_else(|_| panic!("missing module dir {}", dir.display()));
    for e in entries {
        let p = e.unwrap().path();
        if p.is_dir() {
            walk(&p, files);
        } else {
            println!("cargo:rerun-if-changed={}", p.display());
            files.push(p);
        }
    }
}

/// Key/value pairs from the SKILL.md frontmatter block. Indented lines
/// continue the previous value (YAML folded style).
fn frontmatter(name: &str, skill: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut lines = skill.lines();
    if lines.next() != Some("---") {
        panic!("{}: SKILL.md must open with --- frontmatter", name);
    }
    for line in lines {
        if line.trim_end() == "---" {
            return out;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(last) = out.last_mut() {
                if !last.1.is_empty() {
                    last.1.push(' ');
                }
                last.1.push_str(line.trim());
            }
        } else if let Some((k, v)) = line.split_once(':') {
            let v = v.trim();
            out.push((k.trim().to_string(), v.trim_start_matches(">-").trim().to_string()));
        }
    }
    panic!("{}: unterminated frontmatter", name);
}
