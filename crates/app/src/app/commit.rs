//! Committing the staged workspace as a single Git DB commit:
//! blobs → tree (on the head's base tree) → commit → fast-forward the ref.
//! Author/committer/date overrides ride along on the commit object.

use crate::github::{self, GitUser, TreeChange};

use super::{App, CommitIdentity, CommitTarget, Msg, Staged};

impl App {
    /// Open the commit overlay, first folding the open file's latest buffer
    /// into the staged set so "edit one file → commit" still works in a step.
    pub(super) fn begin_commit(&mut self) {
        if self.rv.as_ref().map(|rv| rv.committing).unwrap_or(true) {
            return;
        }
        self.stage_current_file();
        let staged = self.rv.as_ref().map(|rv| rv.staged.len()).unwrap_or(0);
        if staged == 0 {
            self.toast = Some(("nothing staged to commit".into(), false));
            return;
        }
        if self.token.is_none() {
            self.toast = Some(("commits require an access token".into(), true));
            return;
        }
        let form = super::CommitForm::new(&self.commit_identity);
        self.overlay = Some(super::Overlay::Commit(form));
    }

    /// Commit every staged change atomically. `id` overrides are remembered
    /// on the `App`. `target` chooses the destination: the current branch
    /// (fast-forward), or a new branch / tag (`name`) pointed at the commit.
    pub(super) fn commit_staged(
        &mut self,
        message: String,
        id: CommitIdentity,
        target: CommitTarget,
        name: String,
    ) {
        // A date override needs an author name+email to attach to (GitHub
        // rejects a partial author object).
        if !id.date.is_empty() && (id.author_name.is_empty() || id.author_email.is_empty()) {
            self.toast = Some(("set author name + email to override the date".into(), true));
            return;
        }
        if target != CommitTarget::Current && name.is_empty() {
            let kind = if target == CommitTarget::NewTag { "tag" } else { "branch" };
            self.toast = Some((format!("enter a name for the new {}", kind), true));
            return;
        }
        self.commit_identity = id.clone();
        let Some(rv) = self.rv.as_mut() else { return };
        if rv.staged.is_empty() || rv.committing {
            return;
        }
        let Some(head) = rv.branch_sha() else {
            self.toast = Some(("branch head unknown — reload the repo".into(), true));
            return;
        };
        rv.committing = true;
        let changes: Vec<(String, Staged)> =
            rv.staged.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        let current = rv.branch.clone();
        // What the result lands on: the ref name, and whether it's a tag (a
        // tag leaves every branch — and the view — untouched).
        let is_tag = target == CommitTarget::NewTag;
        let landed = if target == CommitTarget::Current { current.clone() } else { name.clone() };
        let author = build_user(&id, false);
        // Committer defaults to the author when only author fields are set.
        let committer = build_user(&id, true).or_else(|| author.clone());
        crate::spawn_msg(async move {
            let result = commit_flow(
                &token, &full, &current, target, &name, &head, &message, &changes, author,
                committer,
            )
            .await;
            Msg::Committed { repo: full, name: landed, is_tag, result }
        });
        self.toast = Some(("committing…".into(), false));
    }
}

/// Build an author/committer identity, or `None` to let GitHub use the
/// token's default. Name + email are both required for an override.
fn build_user(id: &CommitIdentity, committer: bool) -> Option<GitUser> {
    let (name, email) = if committer {
        (&id.committer_name, &id.committer_email)
    } else {
        (&id.author_name, &id.author_email)
    };
    if name.is_empty() || email.is_empty() {
        return None;
    }
    GitUser {
        name: name.clone(),
        email: email.clone(),
        date: (!id.date.is_empty()).then(|| id.date.clone()),
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::{build_user, CommitIdentity};

    #[test]
    fn override_requires_name_and_email() {
        let mut id = CommitIdentity::default();
        // Nothing set → no override (GitHub uses the token identity).
        assert!(build_user(&id, false).is_none());
        // Name without email is incomplete → still none.
        id.author_name = "Ada".into();
        assert!(build_user(&id, false).is_none());
        // Both set → an author override, date carried when present.
        id.author_email = "ada@x.dev".into();
        id.date = "2021-01-01T00:00:00Z".into();
        let u = build_user(&id, false).expect("author");
        assert_eq!(u.name, "Ada");
        assert_eq!(u.email, "ada@x.dev");
        assert_eq!(u.date.as_deref(), Some("2021-01-01T00:00:00Z"));
        // Committer falls back to None when its own fields are blank.
        assert!(build_user(&id, true).is_none());
    }
}

/// The Git DB pipeline. Each blob is created, then a tree on the head's base
/// tree (upserts carry the blob sha; deletes a null sha), then a commit with
/// the current head as its parent. The commit is published per `target`:
/// fast-forward the current branch, or create a new branch / tag ref at it.
#[allow(clippy::too_many_arguments)]
async fn commit_flow(
    token: &Option<String>,
    full: &str,
    current: &str,
    target: CommitTarget,
    name: &str,
    head: &str,
    message: &str,
    changes: &[(String, Staged)],
    author: Option<GitUser>,
    committer: Option<GitUser>,
) -> Result<String, String> {
    let base_tree = github::get_commit(token, full, head).await?.tree.sha;
    let mut tree_changes: Vec<TreeChange> = Vec::with_capacity(changes.len());
    for (path, change) in changes {
        let sha = match change {
            Staged::Upsert(text) => {
                Some(github::create_blob(token, full, text.as_bytes()).await?)
            }
            Staged::Delete => None,
        };
        tree_changes.push(TreeChange { path: path.clone(), sha });
    }
    let tree = github::create_tree(token, full, &base_tree, &tree_changes).await?;
    let commit =
        github::create_commit(token, full, message, &tree, head, author.as_ref(), committer.as_ref())
            .await?;
    match target {
        CommitTarget::Current => github::update_ref(token, full, current, &commit).await?,
        CommitTarget::NewBranch => {
            github::create_ref(token, full, &format!("refs/heads/{}", name), &commit).await?
        }
        CommitTarget::NewTag => {
            github::create_ref(token, full, &format!("refs/tags/{}", name), &commit).await?
        }
    }
    Ok(commit)
}
