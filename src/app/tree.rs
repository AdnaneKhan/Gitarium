//! The file tree: keyboard navigation, row activation, and flattening the
//! recursive tree entries into visible rows.

use std::collections::{HashMap, HashSet};

use crate::github::TreeEntry;
use crate::ui::input::Key;

use super::{App, ConfirmAction, Overlay, RepoView, TreeRow};

impl App {
    pub(super) fn tree_key(&mut self, key: Key) -> bool {
        let Some(rv) = self.rv.as_mut() else { return false };
        let count = rv.rows.len();
        match key {
            Key::Up => rv.tree_sel = rv.tree_sel.saturating_sub(1),
            Key::Down => {
                if count > 0 {
                    rv.tree_sel = (rv.tree_sel + 1).min(count - 1);
                }
            }
            Key::PageUp => rv.tree_sel = rv.tree_sel.saturating_sub(self.layout.tree_h.max(1)),
            Key::PageDown => {
                if count > 0 {
                    rv.tree_sel = (rv.tree_sel + self.layout.tree_h.max(1)).min(count - 1);
                }
            }
            Key::Home => rv.tree_sel = 0,
            Key::End => rv.tree_sel = count.saturating_sub(1),
            Key::Enter | Key::Right => {
                self.activate_tree_row(key == Key::Right);
            }
            Key::Left => {
                let Some(row) = rv.rows.get(rv.tree_sel) else { return false };
                if row.is_dir && rv.expanded.contains(&row.path) {
                    let p = row.path.clone();
                    rv.expanded.remove(&p);
                    rebuild_rows(rv);
                } else if let Some(parent) = row.path.rsplit_once('/').map(|(p, _)| p.to_string()) {
                    if let Some(idx) = rv.rows.iter().position(|r| r.path == parent) {
                        rv.tree_sel = idx;
                    }
                }
            }
            _ => return false,
        }
        true
    }

    pub(super) fn activate_tree_row(&mut self, expand_only: bool) {
        let Some(rv) = self.rv.as_mut() else { return };
        let Some(row) = rv.rows.get(rv.tree_sel) else { return };
        if row.is_dir {
            let p = row.path.clone();
            if rv.expanded.contains(&p) {
                if !expand_only {
                    rv.expanded.remove(&p);
                }
            } else {
                rv.expanded.insert(p);
            }
            rebuild_rows(rv);
        } else if !expand_only {
            let path = row.path.clone();
            let modified = rv.file.as_ref().map(|f| f.editor.modified).unwrap_or(false);
            if modified {
                self.overlay = Some(Overlay::Confirm {
                    msg: "discard unsaved edits and open another file?".into(),
                    action: ConfirmAction::OpenFile(path),
                });
            } else {
                self.open_file(path);
            }
        }
    }
}

/// Flatten the recursive tree entries into visible rows honoring `expanded`.
pub fn rebuild_rows(rv: &mut RepoView) {
    let Some(entries) = rv.tree.ready() else {
        rv.rows.clear();
        return;
    };
    // parent dir -> indices of children
    let mut children: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, e) in entries.iter().enumerate() {
        let parent = e.path.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        children.entry(parent).or_default().push(i);
    }
    for v in children.values_mut() {
        v.sort_by(|&a, &b| {
            let (ea, eb) = (&entries[a], &entries[b]);
            (eb.kind == "tree")
                .cmp(&(ea.kind == "tree"))
                .then_with(|| ea.path.to_lowercase().cmp(&eb.path.to_lowercase()))
        });
    }
    let mut rows = Vec::new();
    fn descend(
        entries: &[TreeEntry],
        children: &HashMap<&str, Vec<usize>>,
        expanded: &HashSet<String>,
        dir: &str,
        depth: usize,
        rows: &mut Vec<TreeRow>,
    ) {
        let Some(idxs) = children.get(dir) else { return };
        for &i in idxs {
            let e = &entries[i];
            let name = e.path.rsplit('/').next().unwrap_or(&e.path).to_string();
            let is_dir = e.kind == "tree";
            rows.push(TreeRow { path: e.path.clone(), name, depth, is_dir });
            if is_dir && expanded.contains(&e.path) {
                descend(entries, children, expanded, &e.path, depth + 1, rows);
            }
        }
    }
    descend(entries, &children, &rv.expanded, "", 0, &mut rows);
    rv.rows = rows;
    if rv.tree_sel >= rv.rows.len() {
        rv.tree_sel = rv.rows.len().saturating_sub(1);
    }
}
