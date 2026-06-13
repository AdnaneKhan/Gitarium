//! Async results for file loads and commits — with the staleness guards
//! that keep racing responses from corrupting another view's state.

use crate::highlight::{self, LineState};

use super::editor::Editor;
use super::{App, Loadable, OpenFile};

impl App {
    pub(super) fn on_file_loaded(
        &mut self,
        repo: String,
        branch: String,
        path: String,
        result: Result<(String, Vec<u8>), String>,
    ) {
        let Some(rv) = &mut self.rv else { return };
        // The branch guard stops an old-branch response from winning the
        // race after a switch + reopen of the same path (commits would then
        // target the wrong base sha).
        if rv.repo.full_name != repo
            || rv.branch != branch
            || rv.file_loading.as_deref() != Some(path.as_str())
        {
            return;
        }
        rv.file_loading = None;
        match result {
            Ok((sha, bytes)) => {
                let size = bytes.len() as u64;
                let binary = bytes.contains(&0);
                let text = if binary {
                    String::new()
                } else {
                    match String::from_utf8(bytes) {
                        Ok(t) => t,
                        Err(_) => {
                            rv.file = Some(make_binary_file(path, sha, size));
                            return;
                        }
                    }
                };
                if binary {
                    rv.file = Some(make_binary_file(path, sha, size));
                    return;
                }
                let lang = highlight::lang_for_path(&path);
                let mut file = OpenFile {
                    path,
                    sha,
                    editor: Editor::from_text(&text),
                    lang,
                    line_states: Vec::new(),
                    binary: false,
                    size,
                    editing: false,
                    committing: false,
                    pending_commit: None,
                };
                rehighlight(&mut file);
                rv.file = Some(file);
            }
            Err(e) => self.toast = Some((e, true)),
        }
    }

    pub(super) fn on_committed(
        &mut self,
        repo: String,
        branch: String,
        path: String,
        result: Result<(String, String), String>,
    ) {
        // The toast is always shown (the user should hear about a failed
        // commit even after navigating), but state is only mutated when
        // repo, branch and path all still match — a stale result must not
        // touch another view's sha/head.
        let fresh = self
            .rv
            .as_ref()
            .map(|rv| rv.repo.full_name == repo && rv.branch == branch)
            .unwrap_or(false);
        match &result {
            Ok((_, commit_sha)) => {
                let short: String = commit_sha.chars().take(7).collect();
                self.toast = Some((format!("committed {} ✓", short), false));
            }
            Err(e) => self.toast = Some((format!("commit failed: {}", e), true)),
        }
        if !fresh {
            return;
        }
        let Some(rv) = &mut self.rv else { return };
        let Some(file) = &mut rv.file else { return };
        if file.path != path {
            return;
        }
        file.committing = false;
        let sent = file.pending_commit.take();
        if let Ok((content_sha, commit_sha)) = result {
            file.sha = content_sha;
            // Edits typed while the commit was in flight must stay marked
            // dirty; only an unchanged buffer becomes clean. (`sent` is
            // None when the file object was reloaded since — nothing to
            // compare, leave `modified` alone.)
            match sent {
                Some(s) if s == file.editor.to_text() => file.editor.modified = false,
                Some(_) => {
                    self.toast = Some(("committed ✓ — buffer has newer edits".into(), false));
                }
                None => {}
            }
            // Keep the branch head fresh so tree reloads work.
            if let Loadable::Ready(branches) = &mut rv.branches {
                if let Some(b) = branches.iter_mut().find(|b| b.name == branch) {
                    if !commit_sha.is_empty() {
                        b.commit.sha = commit_sha.clone();
                    }
                }
            }
        }
    }
}

fn make_binary_file(path: String, sha: String, size: u64) -> OpenFile {
    OpenFile {
        path,
        sha,
        editor: Editor::from_text(""),
        lang: None,
        line_states: Vec::new(),
        binary: true,
        size,
        editing: false,
        committing: false,
        pending_commit: None,
    }
}

pub fn rehighlight(file: &mut OpenFile) {
    let n = file.editor.lines.len();
    let Some(spec) = file.lang else {
        file.line_states = vec![LineState::Normal; n];
        return;
    };
    let mut states = Vec::with_capacity(n);
    let mut st = LineState::Normal;
    for line in &file.editor.lines {
        states.push(st);
        st = highlight::highlight(spec, line, st).1;
    }
    file.line_states = states;
}
