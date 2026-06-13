//! The staged workspace: local add / edit / delete changes cached on the
//! `RepoView`, reviewed and committed together (see `commit.rs`). Replaces
//! the one-file-at-a-time Contents API flow with a Git-like staging area.

use crate::ui::lineinput::LineInput;

use super::editor::Editor;
use super::file_msgs::rehighlight;
use super::{App, OpenFile, Overlay, RepoFocus, Staged};

impl App {
    /// Capture the open file's current buffer into the staged set. Returns
    /// true when something was staged (it had edits, or was already staged).
    pub(super) fn stage_current_file(&mut self) -> bool {
        let Some(rv) = self.rv.as_mut() else { return false };
        let Some(f) = rv.file.as_mut() else { return false };
        if f.binary {
            return false;
        }
        if !f.editor.modified && !rv.staged.contains_key(&f.path) {
            return false;
        }
        rv.staged.insert(f.path.clone(), Staged::Upsert(f.editor.to_text()));
        f.editor.modified = false;
        true
    }

    /// Explicit "stage this file" action (Ctrl+S in the editor, STAGE chip).
    pub(super) fn stage_file_action(&mut self) {
        let path = self.rv.as_ref().and_then(|rv| rv.file.as_ref()).map(|f| f.path.clone());
        if self.stage_current_file() {
            let n = self.rv.as_ref().map(|rv| rv.staged.len()).unwrap_or(0);
            let p = path.unwrap_or_default();
            self.toast = Some((format!("staged {} · {} change(s)", p, n), false));
        } else {
            self.toast = Some(("no changes to stage".into(), false));
        }
    }

    /// Stage a path for deletion — or, if it is a not-yet-committed add,
    /// simply drop the staged change.
    pub(super) fn stage_delete(&mut self, path: String) {
        let Some(rv) = self.rv.as_mut() else { return };
        let in_tree = rv
            .tree
            .ready()
            .map(|t| t.iter().any(|e| e.path == path && e.kind == "blob"))
            .unwrap_or(false);
        if in_tree {
            rv.staged.insert(path.clone(), Staged::Delete);
            self.toast = Some((format!("staged delete · {}", path), false));
        } else {
            rv.staged.remove(&path);
            self.toast = Some((format!("unstaged · {}", path), false));
        }
        // Drop the viewer if the deleted path was open.
        if rv.file.as_ref().map(|f| f.path == path).unwrap_or(false) {
            rv.file = None;
            rv.focus = RepoFocus::Tree;
        }
        super::rebuild_rows(rv);
    }

    /// Discard a path's staged change.
    pub(super) fn unstage(&mut self, path: &str) {
        let mut removed = false;
        if let Some(rv) = self.rv.as_mut() {
            if rv.staged.remove(path).is_some() {
                super::rebuild_rows(rv);
                removed = true;
            }
        }
        if removed {
            self.toast = Some((format!("unstaged · {}", path), false));
        }
    }

    /// Open the new-file prompt, optionally pre-filling a directory prefix so
    /// the file lands there (used by the tree's right-click "New file…").
    pub(super) fn begin_new_file(&mut self) {
        self.begin_new_file_in(String::new());
    }

    pub(super) fn begin_new_file_in(&mut self, dir: String) {
        if self.rv.is_none() {
            return;
        }
        let mut input = LineInput::new(false);
        if !dir.is_empty() {
            input.insert(&format!("{}/", dir.trim_end_matches('/')));
        }
        self.overlay = Some(Overlay::NewFile(input));
    }

    /// Create an empty staged file at `path` and open it for editing.
    pub(super) fn create_new_file(&mut self, path: String) {
        let path = path.trim().trim_start_matches('/').to_string();
        if path.is_empty() {
            return;
        }
        let Some(rv) = self.rv.as_mut() else { return };
        let exists = rv.tree.ready().map(|t| t.iter().any(|e| e.path == path)).unwrap_or(false)
            || rv.staged.contains_key(&path);
        if exists {
            self.toast = Some((format!("{} already exists", path), true));
            return;
        }
        rv.staged.insert(path.clone(), Staged::Upsert(String::new()));
        super::rebuild_rows(rv);
        let lang = crate::highlight::lang_for_path(&path);
        let mut file = OpenFile {
            path,
            sha: String::new(),
            editor: Editor::from_text(""),
            lang,
            line_states: Vec::new(),
            binary: false,
            size: 0,
            editing: true,
        };
        file.editor.read_only = false;
        rehighlight(&mut file);
        rv.focus = RepoFocus::Content;
        rv.file = Some(file);
    }
}
