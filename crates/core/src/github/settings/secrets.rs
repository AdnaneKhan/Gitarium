//! Actions secrets: list, the repo public key (to seal-encrypt values), and
//! create/update (PUT) / delete. Secret values are sealed-box-encrypted by the
//! caller (crate::crypto) before `put_secret` sees them.

use super::types::{PublicKeyResp, SecretMeta, SecretsResp};
use super::{api, enc, enc_path, parse};

/// One segment has already exceeded expectations; keep it to 100/page (GitHub's
/// max for this endpoint).
pub async fn list_secrets(token: &Option<String>, full_name: &str) -> Result<Vec<SecretMeta>, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/actions/secrets?per_page=100", enc_path(full_name)),
        token,
        None,
    )
    .await?;
    Ok(parse::<SecretsResp>(s, b)?.secrets)
}

/// The repo's Actions public key — `key` (base64 X25519) + `key_id`.
pub async fn get_public_key(token: &Option<String>, full_name: &str) -> Result<PublicKeyResp, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/actions/secrets/public-key", enc_path(full_name)),
        token,
        None,
    )
    .await?;
    parse(s, b)
}

/// Create or update a secret. `encrypted_value` is the base64 sealed-box
/// ciphertext of the plaintext; `key_id` is from `get_public_key`. 201 on
/// create, 204 on update — both in the 2xx range.
pub async fn put_secret(
    token: &Option<String>,
    full_name: &str,
    name: &str,
    encrypted_value: &str,
    key_id: &str,
) -> Result<(), String> {
    let body = serde_json::json!({ "encrypted_value": encrypted_value, "key_id": key_id });
    let (s, b) = api(
        "PUT",
        &format!("/repos/{}/actions/secrets/{}", enc_path(full_name), enc(name)),
        token,
        Some(body.to_string()),
    )
    .await?;
    ok_or_err(s, b)
}

pub async fn delete_secret(token: &Option<String>, full_name: &str, name: &str) -> Result<(), String> {
    let (s, b) = api(
        "DELETE",
        &format!("/repos/{}/actions/secrets/{}", enc_path(full_name), enc(name)),
        token,
        None,
    )
    .await?;
    ok_or_err(s, b)
}

/// 2xx with an empty/ignored body → Ok; otherwise a one-line error with
/// GitHub's `message` parsed out (so a 403 surfaces "Resource not accessible
/// by personal access token" rather than raw JSON).
pub(super) fn ok_or_err(status: u16, body: String) -> Result<(), String> {
    if (200..300).contains(&status) {
        return Ok(());
    }
    #[derive(serde::Deserialize)]
    struct ApiError {
        #[serde(default)]
        message: String,
    }
    let msg = serde_json::from_str::<ApiError>(&body)
        .map(|e| e.message)
        .unwrap_or_default();
    let msg = if msg.is_empty() {
        body.chars().take(120).collect::<String>()
    } else {
        msg
    };
    Err(format!("HTTP {}: {}", status, msg))
}
