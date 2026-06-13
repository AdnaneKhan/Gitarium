//! The find-file palette over the already-fetched tree, plus the tree-search
//! ranking it shares with the view. (GitHub code search lives in
//! `code_search.rs`.)

use crate::github::TreeEntry;
use crate::ui::input::{Key, Mods};

use super::{App, ConfirmAction, Overlay};

impl App {
    pub(super) fn file_search_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(Overlay::FileSearch { input, sel }) = &mut self.overlay else { return false };
        match key {
            Key::Esc => {
                self.overlay = None;
                true
            }
            Key::Up => {
                *sel = sel.saturating_sub(1);
                true
            }
            Key::Down => {
                let count = self
                    .rv
                    .as_ref()
                    .and_then(|rv| rv.tree.ready())
                    .map(|t| search_tree(t, &input.text).len())
                    .unwrap_or(0);
                if count > 0 {
                    *sel = (*sel + 1).min(count - 1);
                }
                true
            }
            Key::Enter => {
                let path = self.rv.as_ref().and_then(|rv| rv.tree.ready()).and_then(|t| {
                    search_tree(t, &input.text).get(*sel).map(|&i| t[i].path.clone())
                });
                self.overlay = None;
                if let Some(path) = path {
                    let modified = self
                        .rv
                        .as_ref()
                        .and_then(|rv| rv.file.as_ref())
                        .map(|f| f.editor.modified)
                        .unwrap_or(false);
                    if modified {
                        self.overlay = Some(Overlay::Confirm {
                            msg: format!("discard unsaved edits and open {}?", path),
                            action: ConfirmAction::OpenFile(path),
                        });
                    } else {
                        self.open_file(path);
                    }
                }
                true
            }
            k => {
                let used = input.handle_key(&k, mods);
                if used {
                    *sel = 0;
                }
                used
            }
        }
    }

}

/// Search blob paths in the fetched tree: case-insensitive, every
/// whitespace-separated term must match, ranked by match position and path
/// length so filename hits beat deep-path hits.
pub fn search_tree(entries: &[TreeEntry], query: &str) -> Vec<usize> {
    let q = query.to_lowercase();
    let terms: Vec<&str> = q.split_whitespace().collect();
    let mut hits: Vec<(usize, usize)> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind == "blob")
        .filter_map(|(i, e)| {
            let p = e.path.to_lowercase();
            if terms.is_empty() {
                return Some((p.len(), i));
            }
            let mut score = p.len();
            for t in &terms {
                match p.find(t) {
                    Some(pos) => score += pos,
                    None => return None,
                }
            }
            Some((score, i))
        })
        .collect();
    hits.sort_by_key(|h| h.0);
    hits.truncate(200);
    hits.into_iter().map(|(_, i)| i).collect()
}
