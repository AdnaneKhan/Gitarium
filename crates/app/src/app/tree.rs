//! The file tree: keyboard navigation, row activation, and flattening the
//! recursive tree entries into visible rows.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::ui::input::Key;

use super::{App, ConfirmAction, Overlay, RepoView, Staged, TreeRow};

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
            let cur_path = rv.file.as_ref().map(|f| f.path.clone());
            let modified = rv.file.as_ref().map(|f| f.editor.modified).unwrap_or(false);
            let cur_staged = cur_path.map(|p| rv.staged.contains_key(&p)).unwrap_or(false);
            if cur_staged {
                // A staged file's buffer is captured (not discarded) on
                // navigate, so reopening reads the latest content from memory.
                self.stage_current_file();
                self.open_file(path);
            } else if modified {
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

/// Flatten the tree into visible rows honoring `expanded`. The node set is
/// the remote tree merged with the staged workspace: staged *adds* (paths not
/// in the remote tree) are injected — along with any new ancestor directories
/// they need — so uncommitted files are visible and navigable. Staged deletes
/// stay (they are remote) and the renderer marks them.
pub fn rebuild_rows(rv: &mut RepoView) {
    // path -> is_dir, deduped and sorted by BTreeMap insertion.
    let mut nodes: BTreeMap<String, bool> = BTreeMap::new();
    if let Some(entries) = rv.tree.ready() {
        for e in entries {
            if e.kind == "blob" || e.kind == "tree" {
                nodes.insert(e.path.clone(), e.kind == "tree");
            }
        }
    } else if rv.staged.is_empty() {
        rv.rows.clear();
        return;
    }
    for (path, change) in &rv.staged {
        if !matches!(change, Staged::Upsert(_)) || nodes.contains_key(path) {
            continue;
        }
        // Synthesize ancestor directories for a newly-added file.
        let parts: Vec<&str> = path.split('/').collect();
        let mut acc = String::new();
        for p in &parts[..parts.len().saturating_sub(1)] {
            if !acc.is_empty() {
                acc.push('/');
            }
            acc.push_str(p);
            nodes.entry(acc.clone()).or_insert(true);
        }
        nodes.insert(path.clone(), false);
    }

    let items: Vec<(String, bool)> = nodes.into_iter().collect();
    // parent dir -> child indices
    let mut children: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, (path, _)) in items.iter().enumerate() {
        let parent = path.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        children.entry(parent).or_default().push(i);
    }
    for v in children.values_mut() {
        v.sort_by(|&a, &b| {
            let (pa, da) = &items[a];
            let (pb, db) = &items[b];
            db.cmp(da).then_with(|| pa.to_lowercase().cmp(&pb.to_lowercase()))
        });
    }
    let mut rows = Vec::new();
    fn descend(
        items: &[(String, bool)],
        children: &HashMap<&str, Vec<usize>>,
        expanded: &HashSet<String>,
        dir: &str,
        depth: usize,
        rows: &mut Vec<TreeRow>,
    ) {
        let Some(idxs) = children.get(dir) else { return };
        for &i in idxs {
            let (path, is_dir) = &items[i];
            let name = path.rsplit('/').next().unwrap_or(path).to_string();
            rows.push(TreeRow { path: path.clone(), name, depth, is_dir: *is_dir });
            if *is_dir && expanded.contains(path) {
                descend(items, children, expanded, path, depth + 1, rows);
            }
        }
    }
    descend(&items, &children, &rv.expanded, "", 0, &mut rows);
    rv.rows = rows;
    if rv.tree_sel >= rv.rows.len() {
        rv.tree_sel = rv.rows.len().saturating_sub(1);
    }
}
