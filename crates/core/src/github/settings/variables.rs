//! Actions variables: plaintext (unencrypted) name/value pairs. List, create
//! (POST), update (PATCH), delete.

use super::types::{Variable, VariablesResp};
use super::{api, enc, enc_path, parse};
use super::secrets::ok_or_err;

pub async fn list_variables(token: &Option<String>, full_name: &str) -> Result<Vec<Variable>, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/actions/variables?per_page=100", enc_path(full_name)),
        token,
        None,
    )
    .await?;
    Ok(parse::<VariablesResp>(s, b)?.variables)
}

pub async fn create_variable(
    token: &Option<String>,
    full_name: &str,
    name: &str,
    value: &str,
) -> Result<(), String> {
    let body = serde_json::json!({ "name": name, "value": value });
    let (s, b) = api(
        "POST",
        &format!("/repos/{}/actions/variables", enc_path(full_name)),
        token,
        Some(body.to_string()),
    )
    .await?;
    ok_or_err(s, b)
}

pub async fn update_variable(
    token: &Option<String>,
    full_name: &str,
    name: &str,
    value: &str,
) -> Result<(), String> {
    let body = serde_json::json!({ "name": name, "value": value });
    let (s, b) = api(
        "PATCH",
        &format!("/repos/{}/actions/variables/{}", enc_path(full_name), enc(name)),
        token,
        Some(body.to_string()),
    )
    .await?;
    ok_or_err(s, b)
}

pub async fn delete_variable(token: &Option<String>, full_name: &str, name: &str) -> Result<(), String> {
    let (s, b) = api(
        "DELETE",
        &format!("/repos/{}/actions/variables/{}", enc_path(full_name), enc(name)),
        token,
        None,
    )
    .await?;
    ok_or_err(s, b)
}
