//! Repository-settings endpoints. Phase 1: Actions secrets & variables and
//! deploy keys. Future sections (collaborators, webhooks, environments,
//! rulesets, branch protection, actions permissions, general metadata) will
//! land in their own submodules following the same shape.

mod collaborators;
mod deploy_keys;
mod general;
mod secrets;
mod types;
mod variables;
mod webhooks;

pub use collaborators::{add_collaborator, list_collaborators, remove_collaborator};
pub use deploy_keys::{add_deploy_key, delete_deploy_key, list_deploy_keys};
pub use general::{archive_repo, delete_repo, update_repo};
pub use secrets::{delete_secret, get_public_key, list_secrets, put_secret};
pub use types::*;
pub use variables::{create_variable, delete_variable, list_variables, update_variable};
pub use webhooks::{create_webhook, delete_webhook, list_webhooks, update_webhook};

// Bring the request primitives into `settings` scope so each submodule can
// `use super::{api, enc, enc_path, parse}` instead of reaching two levels up.
use super::{api, enc, enc_path, parse};
