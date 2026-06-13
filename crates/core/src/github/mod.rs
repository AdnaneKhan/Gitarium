//! Minimal GitHub REST v3 client. Every call takes an optional PAT;
//! unauthenticated requests work for public data at 60 req/hour.

mod actions;
mod checks;
mod content;
mod gitdb;
mod issues;
mod pulls;
mod repos;
mod search;
#[cfg(test)]
mod tests;
mod types;

pub use actions::{get_job_logs, list_jobs, list_runs};
pub use checks::{list_check_runs, list_reviews, CheckRun, Review};
pub use content::{get_blob, get_file, get_tree, list_branches, put_file};
pub use gitdb::{
    create_blob, create_commit, create_ref, create_tree, get_commit, update_ref, GitUser,
    TreeChange,
};
pub use issues::{list_comments, list_issues, Comment, Issue, Label};
pub use pulls::{approve_pull, get_pull, list_pulls, merge_pull, Pull};
pub use repos::{current_user, get_repo, repos_first_page, repos_page, RepoPage, MAX_PAGES};
pub use search::{search_code, search_code_global, CodeHit, CodeMatch, CodeSearch, SEARCH_PER_PAGE};
pub use types::*;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::Deserialize;

const API: &str = "https://api.github.com";

pub(super) async fn api(
    method: &str,
    path: &str,
    token: &Option<String>,
    body: Option<String>,
) -> Result<(u16, String), String> {
    api_with_accept(method, path, token, "application/vnd.github+json", body).await
}

pub(super) async fn api_with_accept(
    method: &str,
    path: &str,
    token: &Option<String>,
    accept: &str,
    body: Option<String>,
) -> Result<(u16, String), String> {
    let mut headers: Vec<(&str, String)> = vec![
        ("Accept", accept.to_string()),
        ("X-GitHub-Api-Version", "2022-11-28".to_string()),
    ];
    if let Some(t) = token {
        headers.push(("Authorization", format!("Bearer {}", t)));
    }
    let resp =
        crate::proxy::github_request(method, &format!("{}{}", API, path), &headers, body).await?;
    Ok((resp.status, resp.body))
}

pub(super) fn parse<T: serde::de::DeserializeOwned>(status: u16, body: String) -> Result<T, String> {
    if !(200..300).contains(&status) {
        #[derive(Deserialize)]
        struct ApiError {
            #[serde(default)]
            message: String,
        }
        let msg = serde_json::from_str::<ApiError>(&body)
            .map(|e| e.message)
            .unwrap_or_default();
        let msg = if msg.is_empty() {
            body.chars().take(120).collect()
        } else {
            msg
        };
        return Err(format!("HTTP {}: {}", status, msg));
    }
    serde_json::from_str(&body).map_err(|e| format!("bad API response: {}", e))
}

/// Percent-encode one URL path segment / query value.
pub(super) fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

pub(super) fn enc_path(p: &str) -> String {
    p.split('/').map(enc).collect::<Vec<_>>().join("/")
}

/// GitHub returns base64 with embedded newlines.
pub fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    B64.decode(clean.as_bytes()).map_err(|e| format!("base64: {}", e))
}

pub fn b64_encode(data: &[u8]) -> String {
    B64.encode(data)
}
