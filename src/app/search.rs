//! The two search palettes — find-file over the fetched tree, and GitHub
//! code search — plus the tree-search ranking they share with the view.

use crate::github::{self, TreeEntry};
use crate::ui::input::{Key, Mods};

use super::{App, ConfirmAction, Loadable, Msg, Overlay};

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

    pub(super) fn code_search_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(Overlay::CodeSearch { input, sel, searched, results }) = &mut self.overlay
        else {
            return false;
        };
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
                let count = results.ready().map(|h| h.len()).unwrap_or(0);
                if count > 0 {
                    *sel = (*sel + 1).min(count - 1);
                }
                true
            }
            Key::Enter => {
                let q = input.text.trim().to_string();
                if q.is_empty() {
                    return true;
                }
                if q != *searched {
                    // Submit (explicitly — code search is 10 req/min).
                    *searched = q.clone();
                    *results = Loadable::Loading;
                    *sel = 0;
                    let token = self.token.clone();
                    let full = self
                        .rv
                        .as_ref()
                        .map(|rv| rv.repo.full_name.clone())
                        .unwrap_or_default();
                    crate::spawn_msg(async move {
                        let result = github::search_code(&token, &full, &q).await;
                        Msg::CodeSearchDone { repo: full, query: q, result }
                    });
                } else if let Loadable::Ready(hits) = results {
                    let path = hits.get(*sel).map(|h| h.path.clone());
                    if let Some(path) = path {
                        self.overlay = None;
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
                }
                true
            }
            k => input.handle_key(&k, mods),
        }
    }

    pub(super) fn on_code_search_done(
        &mut self,
        repo: String,
        query: String,
        result: Result<Vec<github::CodeHit>, String>,
    ) {
        let current = self.rv.as_ref().map(|rv| rv.repo.full_name.clone());
        if current.as_deref() != Some(repo.as_str()) {
            return;
        }
        // `searched` guard: a reopened overlay (or a newer query) must not
        // be populated by an older search's results.
        if let Some(Overlay::CodeSearch { results, sel, searched, .. }) = &mut self.overlay {
            if *searched != query {
                return;
            }
            *results = match result {
                Ok(h) => Loadable::Ready(h),
                Err(e) => Loadable::Failed(e),
            };
            *sel = 0;
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
