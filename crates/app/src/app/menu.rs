//! The file-tree right-click menu: building the contextual item list for a
//! clicked row (or empty area) and running the chosen action.

use super::{App, ContextMenu, MenuAction, MenuItem, Staged};

impl App {
    /// Open the tree context menu at (`x`, `y`). `hit` is the right-clicked
    /// row as (path, is_dir), or `None` for empty tree space (→ new file at
    /// the repo root). Called by the renderer on a right-click.
    pub fn open_tree_menu(&mut self, x: f32, y: f32, hit: Option<(String, bool)>) {
        let Some(rv) = self.rv.as_ref() else { return };
        let mut items = Vec::new();
        match hit {
            Some((path, is_dir)) => {
                let dir = if is_dir {
                    path.clone()
                } else {
                    path.rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default()
                };
                items.push(MenuItem { label: "New file…".into(), action: MenuAction::NewFile(dir) });
                if !is_dir {
                    let in_tree = rv
                        .tree
                        .ready()
                        .map(|t| t.iter().any(|e| e.path == path && e.kind == "blob"))
                        .unwrap_or(false);
                    match rv.staged.get(&path) {
                        Some(Staged::Delete) => items.push(MenuItem {
                            label: "Unstage delete".into(),
                            action: MenuAction::Unstage(path.clone()),
                        }),
                        Some(Staged::Upsert(_)) => {
                            items.push(MenuItem {
                                label: "Unstage".into(),
                                action: MenuAction::Unstage(path.clone()),
                            });
                            if in_tree {
                                items.push(MenuItem {
                                    label: "Delete".into(),
                                    action: MenuAction::Delete(path.clone()),
                                });
                            }
                        }
                        None => items.push(MenuItem {
                            label: "Delete".into(),
                            action: MenuAction::Delete(path.clone()),
                        }),
                    }
                }
            }
            None => items.push(MenuItem {
                label: "New file…".into(),
                action: MenuAction::NewFile(String::new()),
            }),
        }
        self.context_menu = Some(ContextMenu { x, y, items });
    }

    /// Run the menu item at `index` (resolved against the open menu) and close.
    /// Called by the renderer when a menu item is clicked.
    pub fn menu_action_at(&mut self, index: usize) {
        let action = self
            .context_menu
            .take()
            .and_then(|m| m.items.into_iter().nth(index))
            .map(|it| it.action);
        match action {
            Some(MenuAction::NewFile(dir)) => self.begin_new_file_in(dir),
            Some(MenuAction::Delete(p)) => self.stage_delete(p),
            Some(MenuAction::Unstage(p)) => self.unstage(&p),
            None => {}
        }
    }
}
