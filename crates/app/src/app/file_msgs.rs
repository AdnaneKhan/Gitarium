//! Async results for file loads and commits — with the staleness guards
//! that keep racing responses from corrupting another view's state.

use crate::github::{Branch, CommitRef};
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
                // A path edited/added in the staged set shows its staged
                // buffer, not the freshly-fetched remote contents.
                let staged_text = rv.staged.get(&path).and_then(|s| match s {
                    super::Staged::Upsert(t) => Some(t.clone()),
                    super::Staged::Delete => None,
                });
                let staged = staged_text.is_some();
                let mut file = OpenFile {
                    path,
                    sha,
                    editor: Editor::from_text(staged_text.as_deref().unwrap_or(&text)),
                    lang,
                    line_states: Vec::new(),
                    binary: false,
                    size,
                    editing: false,
                };
                // Staged content is already captured, so it isn't "modified".
                if staged {
                    file.editor.modified = false;
                }
                rehighlight(&mut file);
                rv.file = Some(file);
            }
            Err(e) => self.toast = Some((e, true)),
        }
    }

    /// A staged commit finished. The toast always fires (the user should
    /// hear about a failure even after navigating); state is mutated when the
    /// repo still matches. A branch landing (current or new) switches the
    /// view to it; a tag landing leaves the current branch — and the view —
    /// untouched (the commit is reachable only via the tag).
    pub(super) fn on_committed(
        &mut self,
        repo: String,
        name: String,
        is_tag: bool,
        result: Result<String, String>,
    ) {
        match &result {
            Ok(commit_sha) => {
                let short: String = commit_sha.chars().take(7).collect();
                let what = if is_tag { "tag" } else { "branch" };
                self.toast = Some((format!("committed {} → {} {} ✓", short, what, name), false));
            }
            Err(e) => self.toast = Some((format!("commit failed: {}", e), true)),
        }
        let same_repo = self.rv.as_ref().map(|rv| rv.repo.full_name == repo).unwrap_or(false);
        if !same_repo {
            return;
        }
        let Ok(commit_sha) = result else {
            if let Some(rv) = &mut self.rv {
                rv.committing = false;
            }
            return;
        };
        {
            let Some(rv) = &mut self.rv else { return };
            rv.committing = false;
            rv.staged.clear();
            if is_tag {
                // The tag doesn't move any branch; just drop the staged rows
                // (the current branch's tree is unchanged).
                super::rebuild_rows(rv);
                return;
            }
            // Record the (possibly new) target branch's head, then make it
            // the active branch so the tree reload uses the committed tree.
            if let Loadable::Ready(branches) = &mut rv.branches {
                match branches.iter_mut().find(|b| b.name == name) {
                    Some(b) => b.commit.sha = commit_sha.clone(),
                    None => branches.push(Branch {
                        name: name.clone(),
                        commit: CommitRef { sha: commit_sha.clone() },
                    }),
                }
            }
            rv.branch = name;
        }
        // Reload the tree from the new head so adds appear and deletes vanish.
        self.load_tree();
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
