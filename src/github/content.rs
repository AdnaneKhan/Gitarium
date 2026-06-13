//! Repo contents: branches, trees, files, blobs, and single-file commits.

use super::types::{Blob, Branch, ContentFile, PutResp, TreeResp};
use super::{api, enc, enc_path, parse};

pub async fn list_branches(token: &Option<String>, full_name: &str) -> Result<Vec<Branch>, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/branches?per_page=100", enc_path(full_name)),
        token,
        None,
    )
    .await?;
    parse(s, b)
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
