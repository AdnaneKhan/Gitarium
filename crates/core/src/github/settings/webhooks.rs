//! Repository webhooks: list, create (POST with config + events), update
//! (PATCH), delete. Events are an array of event names ("push",
//! "pull_request", …) — selected in the UI from a fixed catalog.

use super::types::Webhook;
use super::{api, enc_path, parse};
use super::secrets::ok_or_err;

pub async fn list_webhooks(token: &Option<String>, full_name: &str) -> Result<Vec<Webhook>, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/hooks?per_page=100", enc_path(full_name)),
        token,
        None,
    )
    .await?;
    parse(s, b) // bare array
}

pub async fn create_webhook(
    token: &Option<String>,
    full_name: &str,
    url: &str,
    content_type: &str,
    events: &[String],
    active: bool,
) -> Result<(), String> {
    let body = serde_json::json!({
        "config": { "url": url, "content_type": content_type },
        "events": events,
        "active": active,
    });
    let (s, b) = api(
        "POST",
        &format!("/repos/{}/hooks", enc_path(full_name)),
        token,
        Some(body.to_string()),
    )
    .await?;
    ok_or_err(s, b)
}

pub async fn update_webhook(
    token: &Option<String>,
    full_name: &str,
    id: i64,
    url: &str,
    content_type: &str,
    events: &[String],
    active: bool,
) -> Result<(), String> {
    let body = serde_json::json!({
        "config": { "url": url, "content_type": content_type },
        "events": events,
        "active": active,
    });
    let (s, b) = api(
        "PATCH",
        &format!("/repos/{}/hooks/{}", enc_path(full_name), id),
        token,
        Some(body.to_string()),
    )
    .await?;
    ok_or_err(s, b)
}

pub async fn delete_webhook(token: &Option<String>, full_name: &str, id: i64) -> Result<(), String> {
    let (s, b) = api(
        "DELETE",
        &format!("/repos/{}/hooks/{}", enc_path(full_name), id),
        token,
        None,
    )
    .await?;
    ok_or_err(s, b)
}
