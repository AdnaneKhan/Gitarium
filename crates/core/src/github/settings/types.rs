//! Response types for the repository-settings endpoints: Actions secrets and
//! variables, deploy keys, and the repo public key used to encrypt secrets.
//! The secrets API never returns secret *values* — only names and timestamps.

use serde::Deserialize;

/// A repository-level Actions secret. `value` is intentionally absent: GitHub
/// returns only the name and timestamps.
#[derive(Deserialize, Clone, Debug)]
pub struct SecretMeta {
    pub name: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct SecretsResp {
    #[serde(default)]
    pub total_count: u64,
    #[serde(default)]
    pub secrets: Vec<SecretMeta>,
}

/// A plaintext Actions variable (name + value).
#[derive(Deserialize, Clone, Debug)]
pub struct Variable {
    pub name: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Deserialize)]
pub struct VariablesResp {
    #[serde(default)]
    pub total_count: u64,
    #[serde(default)]
    pub variables: Vec<Variable>,
}

/// A deploy (SSH) key granted repository access. `read_only` keys can pull but
/// not push — a quiet persistence signal worth surfacing.
#[derive(Deserialize, Clone, Debug)]
pub struct DeployKey {
    pub id: i64,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub read_only: bool,
}

/// The repository's Actions public key: `key` is a base64-encoded raw 32-byte
/// X25519 key; `key_id` is sent back alongside the encrypted value.
#[derive(Deserialize, Clone, Debug)]
pub struct PublicKeyResp {
    pub key_id: String,
    pub key: String,
}

/// One section's loaded payload, discriminated by which section was fetched.
/// Lets a single `Msg::SettingsLoaded` carry any section's data.
#[derive(Clone, Debug)]
pub enum SettingsData {
    Secrets(Vec<SecretMeta>),
    Variables(Vec<Variable>),
    DeployKeys(Vec<DeployKey>),
}
