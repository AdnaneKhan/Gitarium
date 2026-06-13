//! Repo contents: branches, trees, files, blobs, and single-file commits.

use super::types::{Blob, Branch, ContentFile, PutResp, TreeResp};
use super::{api, enc, enc_path, parse};

/// Page cap for branch listing: GitHub returns branches alphabetically, so
/// the default branch can sit past the first page. We chain pages until one
/// comes back short, up to 1000 branches — enough to include the default on
/// any realistic repo without hammering pathological ones.
const MAX_BRANCH_PAGES: usize = 10;
const BRANCH_PER_PAGE: usize = 100;

/// All branches (up to the page cap). Fully paginated so the open-repo view
/// finds the repo's `default_branch` instead of falling back to whichever
/// branch happens to sort first.
pub async fn list_branches(token: &Option<String>, full_name: &str) -> Result<Vec<Branch>, String> {
    let mut all: Vec<Branch> = Vec::new();
    for page in 1..=MAX_BRANCH_PAGES {
        let (s, b) = api(
            "GET",
            &format!(
                "/repos/{}/branches?per_page={}&page={}",
                enc_path(full_name),
                BRANCH_PER_PAGE,
                page
            ),
            token,
            None,
        )
        .await?;
        let batch: Vec<Branch> = parse(s, b)?;
        let short = batch.len() < BRANCH_PER_PAGE;
        all.extend(batch);
        if short {
            break;
        }
    }
    Ok(all)
}

/// Full recursive tree for a commit sha (use the branch head sha so branch
/// names containing '/' are never a problem).
pub async fn get_tree(token: &Option<String>, full_name: &str, sha: &str) -> Result<TreeResp, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/git/trees/{}?recursive=1", enc_path(full_name), enc(sha)),
        token,
        None,
    )
    .await?;
    parse(s, b)
}

pub async fn get_file(
    token: &Option<String>,
    full_name: &str,
    path: &str,
    branch: &str,
) -> Result<ContentFile, String> {
    let (s, b) = api(
        "GET",
        &format!(
            "/repos/{}/contents/{}?ref={}",
            enc_path(full_name),
            enc_path(path),
            enc(branch)
        ),
        token,
        None,
    )
    .await?;
    parse(s, b)
}

pub async fn get_blob(token: &Option<String>, full_name: &str, sha: &str) -> Result<Blob, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/git/blobs/{}", enc_path(full_name), enc(sha)),
        token,
        None,
    )
    .await?;
    parse(s, b)
}

/// Create or update one file as a commit on `branch`.
pub async fn put_file(
    token: &Option<String>,
    full_name: &str,
    path: &str,
    message: &str,
    content_b64: &str,
    prev_sha: Option<&str>,
    branch: &str,
) -> Result<PutResp, String> {
    let mut body = serde_json::json!({
        "message": message,
        "content": content_b64,
        "branch": branch,
    });
    if let Some(sha) = prev_sha {
        body["sha"] = serde_json::Value::String(sha.to_string());
    }
    let (s, b) = api(
        "PUT",
        &format!("/repos/{}/contents/{}", enc_path(full_name), enc_path(path)),
        token,
        Some(body.to_string()),
    )
    .await?;
    parse(s, b)
}
