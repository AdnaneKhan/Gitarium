//! A PR's merge requirements: submitted reviews and CI check runs for its
//! head commit. (Legacy commit statuses are not fetched; modern repos report
//! through the Checks API, and `Pull::mergeable_state` already summarizes
//! whether checks are blocking.)

use serde::Deserialize;

use super::types::User;
use super::{api, enc, enc_path, parse};

#[derive(Deserialize, Clone, Debug)]
pub struct Review {
    #[serde(default)]
    pub user: Option<User>,
    /// APPROVED | CHANGES_REQUESTED | COMMENTED | DISMISSED | PENDING
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub submitted_at: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CheckRun {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub status: String, // queued | in_progress | completed
    #[serde(default)]
    pub conclusion: Option<String>, // success | failure | neutral | ...
}

#[derive(Deserialize)]
struct CheckRunsResp {
    #[serde(default)]
    check_runs: Vec<CheckRun>,
}

/// All submitted reviews on a PR (latest state per reviewer is derived in the
/// UI). Up to 100 — enough for the approval summary.
pub async fn list_reviews(
    token: &Option<String>,
    full_name: &str,
    number: u64,
) -> Result<Vec<Review>, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/pulls/{}/reviews?per_page=100", enc_path(full_name), number),
        token,
        None,
    )
    .await?;
    parse(s, b)
}

/// CI check runs for a commit (use the PR's head sha).
pub async fn list_check_runs(
    token: &Option<String>,
    full_name: &str,
    sha: &str,
) -> Result<Vec<CheckRun>, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/commits/{}/check-runs?per_page=100", enc_path(full_name), enc(sha)),
        token,
        None,
    )
    .await?;
    let r: CheckRunsResp = parse(s, b)?;
    Ok(r.check_runs)
}
