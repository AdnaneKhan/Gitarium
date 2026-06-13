//! Opening files for viewing/editing. Staging and the Git DB commit flow
//! live in `staging.rs` / `commit.rs`.

use crate::github;

use super::editor::Editor;
use super::file_msgs::rehighlight;
use super::{App, Msg, OpenFile, RepoFocus, Staged};

impl App {
    pub(super) fn open_file(&mut self, path: String) {
        let Some(rv) = &mut self.rv else { return };
        // Staged adds/edits live only in memory — a new file isn't on the
        // remote, so re-fetching it would 404. Open the staged buffer instead.
        if let Some(Staged::Upsert(text)) = rv.staged.get(&path) {
            let text = text.clone();
            let sha = rv
                .tree
                .ready()
                .and_then(|t| t.iter().find(|e| e.path == path && e.kind == "blob"))
                .map(|e| e.sha.clone())
                .unwrap_or_default();
            let lang = crate::highlight::lang_for_path(&path);
            let mut file = OpenFile {
                path,
                sha,
                editor: Editor::from_text(&text),
                lang,
                line_states: Vec::new(),
                binary: false,
                size: text.len() as u64,
                editing: false,
            };
            rehighlight(&mut file);
            rv.file = Some(file);
            rv.file_loading = None;
            rv.focus = RepoFocus::Content;
            return;
        }
        rv.file = None;
        rv.file_loading = Some(path.clone());
        rv.focus = RepoFocus::Content;
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        let branch = rv.branch.clone();
        crate::spawn_msg(async move {
            let result: Result<(String, Vec<u8>), String> = async {
                let cf = github::get_file(&token, &full, &path, &branch).await?;
                let b64 = match cf.content.as_deref() {
                    Some(c) if !c.trim().is_empty() => c.to_string(),
                    _ if cf.size == 0 => String::new(),
                    _ => github::get_blob(&token, &full, &cf.sha).await?.content,
                };
                let bytes = if b64.is_empty() {
                    Vec::new()
                } else {
                    github::b64_decode(&b64)?
                };
                Ok((cf.sha, bytes))
            }
            .await;
            Msg::FileLoaded { repo: full.clone(), branch, path, result }
        });
    }

    /// Whether the current repo can be edited: an authenticated (non-anon)
    /// session with push access. GitHub omits `permissions` from anonymous
    /// responses, so it's absent → view-only. Gates the EDIT affordance.
    pub fn can_edit_repo(&self) -> bool {
        if self.login.is_none() {
            return false; // anonymous
        }
        self.rv
            .as_ref()
            .and_then(|rv| rv.repo.permissions.as_ref())
            .map(|p| p.push)
            .unwrap_or(false)
    }

    pub(super) fn begin_edit(&mut self) {
        if !self.can_edit_repo() {
            let msg = if self.login.is_none() {
                "sign in to edit this repo"
            } else {
                "view-only: no write access to this repo"
            };
            self.toast = Some((msg.into(), true));
            return;
        }
        let Some(rv) = self.rv.as_mut() else { return };
        if let Some(f) = &mut rv.file {
            if f.binary {
                self.toast = Some(("cannot edit a binary file".into(), true));
                return;
            }
            f.editing = true;
            f.editor.read_only = false;
            rv.focus = RepoFocus::Content;
        }
    }
}
