//! In-memory virtual filesystem backing the agent's bash environment.
//! Holds stored GitHub API responses (/r1.json, /r2.txt, …) and any
//! scratch files the agent writes; nothing ever touches a real disk.

use std::cell::RefCell;
use std::collections::BTreeMap;

thread_local! {
    /// (response counter, path → contents)
    static FS: RefCell<(u32, BTreeMap<String, String>)> = RefCell::new((0, BTreeMap::new()));
}

/// Normalize a path against "/": collapse `.`/`..`, ensure a leading slash.
pub fn norm(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    format!("/{}", parts.join("/"))
}

/// True for paths under the read-only knowledge mount (see
/// docs/knowledge-modules.md): baked in at build time, never writable.
fn read_only(path: &str) -> bool {
    path == "/knowledge" || path.starts_with("/knowledge/")
}

/// Write `content` to `path`; false (and unchanged) if the path is read-only.
pub fn write(path: &str, content: String) -> bool {
    let p = norm(path);
    if read_only(&p) {
        return false;
    }
    FS.with(|f| {
        f.borrow_mut().1.insert(p, content);
    });
    true
}

/// Append to `path`; false (and unchanged) if the path is read-only.
pub fn append(path: &str, content: &str) -> bool {
    let p = norm(path);
    if read_only(&p) {
        return false;
    }
    FS.with(|f| {
        f.borrow_mut().1.entry(p).or_default().push_str(content);
    });
    true
}

/// Install a knowledge file, bypassing the read-only guard (startup only).
pub(crate) fn seed(path: &str, content: String) {
    FS.with(|f| {
        f.borrow_mut().1.insert(norm(path), content);
    });
}

pub fn read(path: &str) -> Option<String> {
    FS.with(|f| f.borrow().1.get(&norm(path)).cloned())
}

pub fn exists(path: &str) -> bool {
    FS.with(|f| f.borrow().1.contains_key(&norm(path)))
}

pub fn remove(path: &str) -> bool {
    let p = norm(path);
    !read_only(&p) && FS.with(|f| f.borrow_mut().1.remove(&p).is_some())
}

/// All (path, char length) pairs, sorted by path.
pub fn list() -> Vec<(String, usize)> {
    FS.with(|f| {
        f.borrow()
            .1
            .iter()
            .map(|(k, v)| (k.clone(), v.chars().count()))
            .collect()
    })
}

/// Reset scratch state; the read-only knowledge mount survives.
pub fn clear() {
    FS.with(|f| {
        let mut f = f.borrow_mut();
        f.0 = 0;
        f.1.retain(|k, _| read_only(k));
    });
}

/// Store an API response as /rN.<ext>, returning the path.
pub fn store_response(body: &str, ext: &str) -> String {
    FS.with(|f| {
        let mut f = f.borrow_mut();
        f.0 += 1;
        let name = format!("/r{}.{}", f.0, ext);
        f.1.insert(name.clone(), body.to_string());
        name
    })
}
