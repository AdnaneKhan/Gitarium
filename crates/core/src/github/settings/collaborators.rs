//! Repository collaborators: list (direct), add/update (PUT with a permission),
//! remove. `affiliation=direct` lists collaborators added directly (the ones a
//! viewer can actually remove), excluding org/team-inherited access.

use super::types::Collaborator;
use super::{api, enc, enc_path, parse};
use super::secrets::ok_or_err;

pub async fn list_collaborators(token: &Option<String>, full_name: &str) -> Result<Vec<Collaborator>, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/collaborators?per_page=100&affiliation=direct", enc_path(full_name)),
        token,
        None,
    )
    .await?;
    parse(s, b) // bare array
}

/// Invite `user` (or update their role) with `permission` ("read" | "triage" |
/// "write" | "maintain" | "admin"). PUT is idempotent: 201 on invite, 204 if
/// already a collaborator — both 2xx.
pub async fn add_collaborator(
    token: &Option<String>,
    full_name: &str,
    user: &str,
    permission: &str,
) -> Result<(), String> {
    let body = serde_json::json!({ "permission": permission });
    let (s, b) = api(
        "PUT",
        &format!("/repos/{}/collaborators/{}", enc_path(full_name), enc(user)),
        token,
        Some(body.to_string()),
    )
    .await?;
    ok_or_err(s, b)
}

pub async fn remove_collaborator(token: &Option<String>, full_name: &str, user: &str) -> Result<(), String> {
    let (s, b) = api(
        "DELETE",
        &format!("/repos/{}/collaborators/{}", enc_path(full_name), enc(user)),
        token,
        None,
    )
    .await?;
    ok_or_err(s, b)
}
