//! Issues and the conversation comments shared by issues and PRs. The
//! `/issues` endpoint returns pull requests too (they carry a
//! `pull_request` link); `list_issues` drops those so the Issues view shows
//! only true issues.

use serde::Deserialize;

use super::types::User;
use super::{api, enc_path, parse};

#[derive(Deserialize, Clone, Debug)]
pub struct Issue {
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
    pub comments: i64,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub labels: Vec<Label>,
    /// Present only when this "issue" is really a pull request — used to
    /// filter PRs out of the issues list.
    #[serde(default)]
    pub pull_request: Option<serde_json::Value>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Comment {
    #[serde(default)]
    pub user: Option<User>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub created_at: String,
}

/// An issue/PR label. `color` is GitHub's 6-hex-digit string (no leading `#`).
#[derive(Deserialize, Clone, Debug)]
pub struct Label {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: String,
}

/// The 100 most-recently-updated open issues (PRs filtered out).
pub async fn list_issues(token: &Option<String>, full_name: &str) -> Result<Vec<Issue>, String> {
    let (s, b) = api(
        "GET",
        &format!(
            "/repos/{}/issues?state=open&sort=updated&direction=desc&per_page=100",
            enc_path(full_name)
        ),
        token,
        None,
    )
    .await?;
    let mut issues: Vec<Issue> = parse(s, b)?;
    issues.retain(|i| i.pull_request.is_none());
    Ok(issues)
}

/// Conversation comments on an issue or PR (same endpoint for both).
pub async fn list_comments(
    token: &Option<String>,
    full_name: &str,
    number: u64,
) -> Result<Vec<Comment>, String> {
    let (s, b) = api(
        "GET",
        &format!(
            "/repos/{}/issues/{}/comments?per_page=100",
            enc_path(full_name),
            number
        ),
        token,
        None,
    )
    .await?;
    parse(s, b)
}
