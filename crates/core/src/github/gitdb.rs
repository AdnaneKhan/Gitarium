//! Git Database API: the low-level blob/tree/commit/ref objects behind the
//! staged-workspace commit flow. Unlike the Contents API (one file = one
//! commit), these compose into a single commit touching many paths at once,
//! with the author, committer, and dates fully under the caller's control.

use super::types::{CommitObj, ObjResp};
use super::{api, b64_encode, enc_path, parse};

/// Create a blob from raw bytes (sent base64, so any content is safe).
/// Returns the new blob's sha.
pub async fn create_blob(
    token: &Option<String>,
    full: &str,
    content: &[u8],
) -> Result<String, String> {
    let body = serde_json::json!({ "content": b64_encode(content), "encoding": "base64" });
    let (s, b) = api(
        "POST",
        &format!("/repos/{}/git/blobs", enc_path(full)),
        token,
        Some(body.to_string()),
    )
    .await?;
    Ok(parse::<ObjResp>(s, b)?.sha)
}

/// Fetch a commit object — we need its `tree.sha` as the base tree for the
/// new commit so unchanged paths are inherited instead of re-uploaded.
pub async fn get_commit(token: &Option<String>, full: &str, sha: &str) -> Result<CommitObj, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/git/commits/{}", enc_path(full), super::enc(sha)),
        token,
        None,
    )
    .await?;
    parse(s, b)
}

/// One change in a new tree: `sha = Some(blob)` upserts the path, `None`
/// deletes it from the base tree.
pub struct TreeChange {
    pub path: String,
    pub sha: Option<String>,
}

/// One tree entry for the create-tree payload. A `None` sha serializes to
/// JSON `null`, which removes the path from the base tree (a deletion);
/// `Some(sha)` upserts the blob at `path`.
fn tree_entry(c: &TreeChange) -> serde_json::Value {
    serde_json::json!({ "path": c.path, "mode": "100644", "type": "blob", "sha": c.sha })
}

/// Build a new tree from `base_tree` plus the given changes; returns its sha.
pub async fn create_tree(
    token: &Option<String>,
    full: &str,
    base_tree: &str,
    changes: &[TreeChange],
) -> Result<String, String> {
    let entries: Vec<serde_json::Value> = changes.iter().map(tree_entry).collect();
    let body = serde_json::json!({ "base_tree": base_tree, "tree": entries });
    let (s, b) = api(
        "POST",
        &format!("/repos/{}/git/trees", enc_path(full)),
        token,
        Some(body.to_string()),
    )
    .await?;
    Ok(parse::<ObjResp>(s, b)?.sha)
}

/// An author/committer identity. `date` is ISO 8601 (RFC 3339); `None` lets
/// GitHub stamp the current time.
#[derive(Clone)]
pub struct GitUser {
    pub name: String,
    pub email: String,
    pub date: Option<String>,
}

fn user_json(u: &GitUser) -> serde_json::Value {
    let mut m = serde_json::json!({ "name": u.name, "email": u.email });
    if let Some(d) = &u.date {
        m["date"] = serde_json::Value::String(d.clone());
    }
    m
}

/// Create a commit pointing at `tree` with `parent` as its sole parent.
/// `author`/`committer` override the defaults GitHub would derive from the
/// token; either left `None` falls back to that default.
pub async fn create_commit(
    token: &Option<String>,
    full: &str,
    message: &str,
    tree: &str,
    parent: &str,
    author: Option<&GitUser>,
    committer: Option<&GitUser>,
) -> Result<String, String> {
    let mut body = serde_json::json!({
        "message": message,
        "tree": tree,
        "parents": [parent],
    });
    if let Some(a) = author {
        body["author"] = user_json(a);
    }
    if let Some(c) = committer {
        body["committer"] = user_json(c);
    }
    let (s, b) = api(
        "POST",
        &format!("/repos/{}/git/commits", enc_path(full)),
        token,
        Some(body.to_string()),
    )
    .await?;
    Ok(parse::<ObjResp>(s, b)?.sha)
}

/// Fast-forward `branch` to `commit_sha` (no force — the commit's parent is
/// the current head, so a non-ff means someone else pushed; surfacing that
/// error is correct).
pub async fn update_ref(
    token: &Option<String>,
    full: &str,
    branch: &str,
    commit_sha: &str,
) -> Result<(), String> {
    let body = serde_json::json!({ "sha": commit_sha, "force": false });
    let (s, b) = api(
        "PATCH",
        &format!("/repos/{}/git/refs/heads/{}", enc_path(full), enc_path(branch)),
        token,
        Some(body.to_string()),
    )
    .await?;
    parse::<serde_json::Value>(s, b).map(|_| ())
}

/// Create a new ref at `commit_sha`. `full_ref` is the fully-qualified name,
/// e.g. `refs/heads/feature` for a branch or `refs/tags/v1` for a tag —
/// used by "commit to a new branch" in the staging flow.
pub async fn create_ref(
    token: &Option<String>,
    full: &str,
    full_ref: &str,
    commit_sha: &str,
) -> Result<(), String> {
    let body = serde_json::json!({ "ref": full_ref, "sha": commit_sha });
    let (s, b) = api(
        "POST",
        &format!("/repos/{}/git/refs", enc_path(full)),
        token,
        Some(body.to_string()),
    )
    .await?;
    parse::<serde_json::Value>(s, b).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_entry_upsert_and_delete() {
        let up = tree_entry(&TreeChange { path: "a/b.rs".into(), sha: Some("deadbeef".into()) });
        assert_eq!(up["path"], "a/b.rs");
        assert_eq!(up["mode"], "100644");
        assert_eq!(up["sha"], "deadbeef");
        // A deletion must send an explicit null sha, not omit the field.
        let del = tree_entry(&TreeChange { path: "gone.txt".into(), sha: None });
        assert!(del["sha"].is_null(), "{}", del);
    }
}
