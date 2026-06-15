//! Deploy (SSH) keys: list (bare array), add (POST with title/key/read_only),
//! delete.

use super::types::DeployKey;
use super::{api, enc_path, parse};
use super::secrets::ok_or_err;

pub async fn list_deploy_keys(token: &Option<String>, full_name: &str) -> Result<Vec<DeployKey>, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/keys?per_page=100", enc_path(full_name)),
        token,
        None,
    )
    .await?;
    parse(s, b) // bare array, no wrapper
}

pub async fn add_deploy_key(
    token: &Option<String>,
    full_name: &str,
    title: &str,
    key: &str,
    read_only: bool,
) -> Result<(), String> {
    let body = serde_json::json!({ "title": title, "key": key, "read_only": read_only });
    let (s, b) = api(
        "POST",
        &format!("/repos/{}/keys", enc_path(full_name)),
        token,
        Some(body.to_string()),
    )
    .await?;
    ok_or_err(s, b)
}

pub async fn delete_deploy_key(token: &Option<String>, full_name: &str, id: i64) -> Result<(), String> {
    let (s, b) = api(
        "DELETE",
        &format!("/repos/{}/keys/{}", enc_path(full_name), id),
        token,
        None,
    )
    .await?;
    ok_or_err(s, b)
}
