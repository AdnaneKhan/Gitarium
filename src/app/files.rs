//! Opening files for viewing/editing and committing them back.

use crate::github;
use crate::ui::lineinput::LineInput;

use super::{App, Msg, Overlay, RepoFocus};

impl App {
    pub(super) fn open_file(&mut self, path: String) {
        let Some(rv) = &mut self.rv else { return };
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

    pub(super) fn commit_file(&mut self, message: String) {
        let Some(rv) = &mut self.rv else { return };
        let Some(file) = &mut rv.file else { return };
        file.committing = true;
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        let path = file.path.clone();
        let branch = rv.branch.clone();
        let sha = file.sha.clone();
        let text = file.editor.to_text();
        file.pending_commit = Some(text.clone());
        let content = github::b64_encode(text.as_bytes());
        crate::spawn_msg(async move {
            let result: Result<(String, String), String> = async {
                let resp =
                    github::put_file(&token, &full, &path, &message, &content, Some(&sha), &branch)
                        .await?;
                let csha = resp.content.map(|c| c.sha).ok_or("no content sha in response")?;
                let ksha = resp.commit.map(|c| c.sha).unwrap_or_default();
                Ok((csha, ksha))
            }
            .await;
            Msg::Committed { repo: full, branch, path, result }
        });
        self.toast = Some(("committing…".into(), false));
    }

    pub(super) fn begin_edit(&mut self) {
        let anonymous = self.token.is_none();
        let Some(rv) = self.rv.as_mut() else { return };
        if let Some(f) = &mut rv.file {
            if f.binary {
                self.toast = Some(("cannot edit a binary file".into(), true));
                return;
            }
            f.editing = true;
            f.editor.read_only = false;
            rv.focus = RepoFocus::Content;
            if anonymous {
                self.toast = Some((
                    "anonymous mode: editing locally, commits will be rejected".into(),
                    false,
                ));
            }
        }
    }

    pub(super) fn begin_commit(&mut self) {
        let Some(rv) = self.rv.as_mut() else { return };
        let Some(f) = &rv.file else { return };
        if f.committing {
            return;
        }
        if !f.editor.modified {
            self.toast = Some(("no changes to commit".into(), false));
            return;
        }
        self.overlay = Some(Overlay::Commit(LineInput::new(false)));
    }
}
