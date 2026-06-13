//! Pull requests: the recently-updated list, a single PR's merge state, and
//! the two write actions the UI exposes — approve and merge.

use serde::Deserialize;

use super::issues::Label;
use super::types::User;
use super::{api, enc_path, parse};

/// A pull request. The list endpoint leaves the merge-state fields null;
/// they are filled in only by [`get_pull`] on a single PR (GitHub computes
/// mergeability lazily), so all are optional/defaulted.
#[derive(Deserialize, Clone, Debug)]
pub struct Pull {
    pub number: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub state: String, // open | closed
    #[serde(default)]
    pub user: Option<User>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub labels: Vec<Label>,
    #[serde(default)]
    pub merged: bool,
    /// null while GitHub is still computing it; Some(true/false) once known.
    #[serde(default)]
    pub mergeable: Option<bool>,
    /// clean | dirty | blocked | behind | unstable | draft | unknown
    #[serde(default)]
    pub mergeable_state: String,
    #[serde(default)]
    pub head: Option<GitRef>,
    #[serde(default)]
    pub base: Option<GitRef>,
    #[serde(default)]
    pub additions: i64,
    #[serde(default)]
    pub deletions: i64,
    #[serde(default)]
    pub changed_files: i64,
    #[serde(default)]
    pub comments: i64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct GitRef {
    #[serde(default)]
    pub sha: String,
    #[serde(rename = "ref", default)]
    pub name: String,
}

/// The 100 most-recently-updated open PRs.
pub async fn list_pulls(token: &Option<String>, full_name: &str) -> Result<Vec<Pull>, String> {
    let (s, b) = api(
        "GET",
        &format!(
            "/repos/{}/pulls?state=open&sort=updated&direction=desc&per_page=100",
            enc_path(full_name)
        ),
        token,
        None,
    )
    .await?;
    parse(s, b)
}

/// One PR with its computed merge state (mergeable / mergeable_state / diff
/// stats), which the list endpoint omits.
pub async fn get_pull(token: &Option<String>, full_name: &str, number: u64) -> Result<Pull, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/pulls/{}", enc_path(full_name), number),
        token,
        None,
    )
    .await?;
    parse(s, b)
}

/// Submit an approving review. Requires read access and that the viewer is
/// not the PR author (GitHub rejects self-approval).
pub async fn approve_pull(
    token: &Option<String>,
    full_name: &str,
    number: u64,
) -> Result<(), String> {
    let (s, b) = api(
        "POST",
        &format!("/repos/{}/pulls/{}/reviews", enc_path(full_name), number),
        token,
        Some("{\"event\":\"APPROVE\"}".to_string()),
    )
    .await?;
    let _: serde_json::Value = parse(s, b)?;
    Ok(())
}

/// Merge a PR. `method` is one of "merge" | "squash" | "rebase". Returns
/// GitHub's confirmation message; a non-mergeable PR surfaces as an error.
pub async fn merge_pull(
    token: &Option<String>,
    full_name: &str,
    number: u64,
    method: &str,
) -> Result<String, String> {
    #[derive(Deserialize)]
    struct MergeResp {
        #[serde(default)]
        merged: bool,
        #[serde(default)]
        message: String,
    }
    let body = format!("{{\"merge_method\":\"{}\"}}", method);
    let (s, b) = api(
        "PUT",
        &format!("/repos/{}/pulls/{}/merge", enc_path(full_name), number),
        token,
        Some(body),
    )
    .await?;
    let r: MergeResp = parse(s, b)?;
    if r.merged {
        Ok(if r.message.is_empty() { "merged".into() } else { r.message })
    } else {
        Err(if r.message.is_empty() { "merge failed".into() } else { r.message })
    }
}
