//! General repository metadata: update (PATCH), archive, and delete (danger
//! zone). `update_repo` and `archive_repo` return the fresh `Repo` so the UI
//! can refresh `rv.repo` in place; `delete_repo` just succeeds (the UI then
//! leaves the repo screen).

use crate::github::Repo;

use super::{api, enc_path, parse};
use super::secrets::ok_or_err;

/// PATCH the repo's metadata. Only non-empty fields are sent (so unchanged
/// fields aren't touched); clearing a field isn't supported in this form.
pub async fn update_repo(
    token: &Option<String>,
    full_name: &str,
    name: &str,
    description: &str,
    homepage: &str,
    default_branch: &str,
) -> Result<Repo, String> {
    let mut body = serde_json::json!({});
    if !name.is_empty() {
        body["name"] = serde_json::Value::String(name.to_string());
    }
    if !description.is_empty() {
        body["description"] = serde_json::Value::String(description.to_string());
    }
    if !homepage.is_empty() {
        body["homepage"] = serde_json::Value::String(homepage.to_string());
    }
    if !default_branch.is_empty() {
        body["default_branch"] = serde_json::Value::String(default_branch.to_string());
    }
    let (s, b) = api(
        "PATCH",
        &format!("/repos/{}", enc_path(full_name)),
        token,
        Some(body.to_string()),
    )
    .await?;
    parse(s, b)
}

pub async fn archive_repo(token: &Option<String>, full_name: &str) -> Result<Repo, String> {
    let (s, b) = api(
        "POST",
        &format!("/repos/{}/archive", enc_path(full_name)),
        token,
        None,
    )
    .await?;
    parse(s, b)
}

pub async fn delete_repo(token: &Option<String>, full_name: &str) -> Result<(), String> {
    let (s, b) = api(
        "DELETE",
        &format!("/repos/{}", enc_path(full_name)),
        token,
        None,
    )
    .await?;
    ok_or_err(s, b)
}
