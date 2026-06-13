//! Deserialization types for the REST v3 payloads the app consumes.

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct User {
    pub login: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Repo {
    pub name: String,
    pub full_name: String,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_branch")]
    pub default_branch: String,
    #[serde(default)]
    pub stargazers_count: i64,
    #[serde(default)]
    pub pushed_at: Option<String>,
    // Card metadata — all present in the list endpoints, no extra requests.
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub forks_count: i64,
    #[serde(default)]
    pub open_issues_count: i64,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub fork: bool,
    #[serde(default)]
    pub license: Option<License>,
    /// The viewer's access level. Present only on authenticated responses;
    /// absent (→ None) for anonymous requests, hence view-only.
    #[serde(default)]
    pub permissions: Option<Permissions>,
}

/// The authenticated viewer's access level on a repo. `push` is GitHub's
/// flag for write access (true for write/maintain/admin collaborators).
#[derive(Deserialize, Clone, Debug, Default)]
pub struct Permissions {
    #[serde(default)]
    pub admin: bool,
    #[serde(default)]
    pub maintain: bool,
    #[serde(default)]
    pub push: bool,
    #[serde(default)]
    pub triage: bool,
    #[serde(default)]
    pub pull: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct License {
    #[serde(default)]
    pub spdx_id: Option<String>,
}

fn default_branch() -> String {
    "main".to_string()
}

#[derive(Deserialize, Clone, Debug)]
pub struct Branch {
    pub name: String,
    pub commit: CommitRef,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CommitRef {
    pub sha: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TreeResp {
    #[serde(default)]
    pub tree: Vec<TreeEntry>,
    #[serde(default)]
    pub truncated: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TreeEntry {
    pub path: String,
    #[serde(rename = "type")]
    pub kind: String, // "blob" | "tree" | "commit" (submodule)
    pub sha: String,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ContentFile {
    #[serde(default)]
    pub content: Option<String>,
    pub sha: String,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub size: u64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Blob {
    pub content: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PutResp {
    pub content: Option<ShaOnly>,
    pub commit: Option<ShaOnly>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ShaOnly {
    pub sha: String,
}

/// The `{ "sha": … }` envelope returned by every Git DB create (blob, tree,
/// commit).
#[derive(Deserialize, Clone, Debug)]
pub struct ObjResp {
    pub sha: String,
}

/// A commit object from the Git DB API — we read `tree.sha` to base a new
/// tree on it.
#[derive(Deserialize, Clone, Debug)]
pub struct CommitObj {
    pub tree: ShaOnly,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RunsResp {
    #[serde(default)]
    pub workflow_runs: Vec<Run>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Run {
    pub id: u64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub display_title: Option<String>,
    #[serde(default)]
    pub run_number: i64,
    #[serde(default)]
    pub status: String, // queued | in_progress | completed
    #[serde(default)]
    pub conclusion: Option<String>, // success | failure | cancelled | ...
    #[serde(default)]
    pub head_branch: Option<String>,
    #[serde(default)]
    pub event: String,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct JobsResp {
    #[serde(default)]
    pub jobs: Vec<Job>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Job {
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub conclusion: Option<String>,
    #[serde(default)]
    pub steps: Vec<Step>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Step {
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub conclusion: Option<String>,
}
