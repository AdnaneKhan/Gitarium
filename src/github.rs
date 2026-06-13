//! Minimal GitHub REST v3 client. Every call takes an optional PAT;
//! unauthenticated requests work for public data at 60 req/hour.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::Deserialize;

use crate::fetch;

const API: &str = "https://api.github.com";

async fn api(
    method: &str,
    path: &str,
    token: &Option<String>,
    body: Option<String>,
) -> Result<(u16, String), String> {
    api_with_accept(method, path, token, "application/vnd.github+json", body).await
}

async fn api_with_accept(
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
    let resp = fetch::request(method, &format!("{}{}", API, path), &headers, body).await?;
    Ok((resp.status, resp.body))
}

fn parse<T: serde::de::DeserializeOwned>(status: u16, body: String) -> Result<T, String> {
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
fn enc(s: &str) -> String {
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

fn enc_path(p: &str) -> String {
    p.split('/').map(enc).collect::<Vec<_>>().join("/")
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Endpoints
// ---------------------------------------------------------------------------

pub async fn current_user(token: &Option<String>) -> Result<User, String> {
    let (s, b) = api("GET", "/user", token, None).await?;
    parse(s, b)
}

const PER_PAGE: usize = 100;
/// Runaway guard for pagination (10k repos). Anonymous sessions exhaust
/// their 60 req/hour budget long before this; the loop then returns what
/// it collected so far, flagged as truncated.
const MAX_PAGES: usize = 100;

/// Why a repo listing stopped before the real end of the data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Truncation {
    /// A page after the first failed (rate limit, auth, network); the
    /// payload is that page's error.
    Error(String),
    /// The MAX_PAGES runaway guard tripped; more pages may exist.
    MaxPages,
}

/// A repo listing plus an optional reason it is incomplete. `truncated:
/// None` means every page was fetched; `Some` means the list is partial
/// and the UI should say so.
#[derive(Clone, Debug)]
pub struct RepoList {
    pub repos: Vec<Repo>,
    pub truncated: Option<Truncation>,
}

/// Fetch successive pages of `base` (a path that already has a query
/// string, no `page` param) until a short page signals the end. Pass page 1
/// in `first` when it was already fetched. Errors after the first page
/// (rate limit, network) end the loop with the repos collected so far and
/// the error recorded in `truncated`.
async fn paged_repos(
    token: &Option<String>,
    base: &str,
    first: Option<Vec<Repo>>,
) -> Result<RepoList, String> {
    let mut all: Vec<Repo> = Vec::new();
    let mut page = 1;
    if let Some(f) = first {
        if f.len() < PER_PAGE {
            return Ok(RepoList { repos: f, truncated: None });
        }
        all = f;
        page = 2;
    }
    while page <= MAX_PAGES {
        let batch: Result<Vec<Repo>, String> =
            match api("GET", &format!("{}&page={}", base, page), token, None).await {
                Ok((s, b)) => parse(s, b),
                Err(e) => Err(e),
            };
        match batch {
            Ok(mut batch) => {
                let n = batch.len();
                all.append(&mut batch);
                if n < PER_PAGE {
                    return Ok(RepoList { repos: all, truncated: None });
                }
            }
            Err(e) => {
                if all.is_empty() {
                    return Err(e);
                }
                // Partial result beats dropping everything fetched, but
                // the gap has to stay visible.
                return Ok(RepoList { repos: all, truncated: Some(Truncation::Error(e)) });
            }
        }
        page += 1;
    }
    // Fell off the page cap with the last page still full.
    Ok(RepoList { repos: all, truncated: Some(Truncation::MaxPages) })
}

/// Repos of the authenticated user, with truncation reported.
pub async fn list_repos_full(token: &Option<String>) -> Result<RepoList, String> {
    let base = format!(
        "/user/repos?per_page={}&sort=pushed&affiliation=owner,collaborator,organization_member",
        PER_PAGE
    );
    paged_repos(token, &base, None).await
}

/// What `/users/{owner}` says an account is, to disambiguate the org-repos
/// 404 fallback.
enum OwnerKind {
    User,
    Organization,
    /// 404 — no such account visible to this token.
    Missing,
}

async fn owner_kind(token: &Option<String>, owner: &str) -> Result<OwnerKind, String> {
    #[derive(Deserialize)]
    struct Account {
        #[serde(rename = "type", default)]
        kind: String,
    }
    let (s, b) = api("GET", &format!("/users/{}", enc(owner)), token, None).await?;
    if s == 404 {
        return Ok(OwnerKind::Missing);
    }
    let acct: Account = parse(s, b)?;
    match acct.kind.as_str() {
        "User" => Ok(OwnerKind::User),
        "Organization" => Ok(OwnerKind::Organization),
        other => Err(format!("unexpected account type '{}'", other)),
    }
}

/// All repos of an organization (paginated), with truncation reported.
/// The orgs endpoint 404s both for user accounts and for orgs this token
/// cannot see, so a 404 is disambiguated via `/users/{owner}` (one extra
/// request): a real user account falls back to the public users listing;
/// an existing org (or an unresolvable owner) is an access error, not a
/// silently shorter list.
pub async fn list_owner_repos_full(
    token: &Option<String>,
    owner: &str,
) -> Result<RepoList, String> {
    let org_base =
        format!("/orgs/{}/repos?per_page={}&sort=pushed&type=all", enc(owner), PER_PAGE);
    let (s, b) = api("GET", &format!("{}&page=1", org_base), token, None).await?;
    if s == 404 {
        return match owner_kind(token, owner).await {
            Ok(OwnerKind::User) => {
                let user_base =
                    format!("/users/{}/repos?per_page={}&sort=pushed", enc(owner), PER_PAGE);
                paged_repos(token, &user_base, None).await
            }
            Ok(OwnerKind::Organization) => Err(format!(
                "no access to organization '{}' (token may lack org scope or SSO authorization)",
                owner
            )),
            Ok(OwnerKind::Missing) => Err(format!("'{}' not found (or no access)", owner)),
            Err(e) => Err(format!("cannot resolve owner '{}': {}", owner, e)),
        };
    }
    let first: Vec<Repo> = parse(s, b)?;
    paged_repos(token, &org_base, Some(first)).await
}

pub async fn get_repo(token: &Option<String>, full_name: &str) -> Result<Repo, String> {
    let (s, b) = api("GET", &format!("/repos/{}", enc_path(full_name)), token, None).await?;
    parse(s, b)
}

pub async fn list_branches(token: &Option<String>, full_name: &str) -> Result<Vec<Branch>, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/branches?per_page=100", enc_path(full_name)),
        token,
        None,
    )
    .await?;
    parse(s, b)
}

/// Full recursive tree for a commit sha (use the branch head sha so branch
/// names containing '/' are never a problem).
pub async fn get_tree(token: &Option<String>, full_name: &str, sha: &str) -> Result<TreeResp, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/git/trees/{}?recursive=1", enc_path(full_name), enc(sha)),
        token,
        None,
    )
    .await?;
    parse(s, b)
}

pub async fn get_file(
    token: &Option<String>,
    full_name: &str,
    path: &str,
    branch: &str,
) -> Result<ContentFile, String> {
    let (s, b) = api(
        "GET",
        &format!(
            "/repos/{}/contents/{}?ref={}",
            enc_path(full_name),
            enc_path(path),
            enc(branch)
        ),
        token,
        None,
    )
    .await?;
    parse(s, b)
}

pub async fn get_blob(token: &Option<String>, full_name: &str, sha: &str) -> Result<Blob, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/git/blobs/{}", enc_path(full_name), enc(sha)),
        token,
        None,
    )
    .await?;
    parse(s, b)
}

/// Create or update one file as a commit on `branch`.
pub async fn put_file(
    token: &Option<String>,
    full_name: &str,
    path: &str,
    message: &str,
    content_b64: &str,
    prev_sha: Option<&str>,
    branch: &str,
) -> Result<PutResp, String> {
    let mut body = serde_json::json!({
        "message": message,
        "content": content_b64,
        "branch": branch,
    });
    if let Some(sha) = prev_sha {
        body["sha"] = serde_json::Value::String(sha.to_string());
    }
    let (s, b) = api(
        "PUT",
        &format!("/repos/{}/contents/{}", enc_path(full_name), enc_path(path)),
        token,
        Some(body.to_string()),
    )
    .await?;
    parse(s, b)
}

pub async fn list_runs(token: &Option<String>, full_name: &str) -> Result<Vec<Run>, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/actions/runs?per_page=50", enc_path(full_name)),
        token,
        None,
    )
    .await?;
    let r: RunsResp = parse(s, b)?;
    Ok(r.workflow_runs)
}

pub async fn list_jobs(token: &Option<String>, full_name: &str, run_id: u64) -> Result<Vec<Job>, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/actions/runs/{}/jobs?per_page=100", enc_path(full_name), run_id),
        token,
        None,
    )
    .await?;
    let r: JobsResp = parse(s, b)?;
    Ok(r.jobs)
}

// ---------------------------------------------------------------------------
// Code search (requires a token; searches the default branch only)
// ---------------------------------------------------------------------------

/// One code-search hit: the file path plus the matched line, with the match
/// range in char indices for highlighting.
#[derive(Clone, Debug)]
pub struct CodeHit {
    pub path: String,
    pub line: String,
    pub range: Option<(usize, usize)>,
}

pub async fn search_code(
    token: &Option<String>,
    full_name: &str,
    query: &str,
) -> Result<Vec<CodeHit>, String> {
    #[derive(Deserialize)]
    struct SearchResp {
        #[serde(default)]
        items: Vec<SearchItem>,
    }
    #[derive(Deserialize)]
    struct SearchItem {
        path: String,
        #[serde(default)]
        text_matches: Vec<TextMatch>,
    }
    #[derive(Deserialize)]
    struct TextMatch {
        #[serde(default)]
        fragment: String,
        #[serde(default)]
        matches: Vec<TMatch>,
    }
    #[derive(Deserialize)]
    struct TMatch {
        #[serde(default)]
        indices: Vec<usize>,
    }

    let q = format!("{} repo:{}", query, full_name);
    let (s, b) = api_with_accept(
        "GET",
        &format!("/search/code?q={}&per_page=30", enc(&q)),
        token,
        "application/vnd.github.text-match+json",
        None,
    )
    .await?;
    let resp: SearchResp = parse(s, b)?;
    Ok(resp
        .items
        .into_iter()
        .map(|it| {
            let tm = it.text_matches.into_iter().find(|t| !t.fragment.is_empty());
            let (line, range) = match tm {
                Some(t) => {
                    let byte_range = t
                        .matches
                        .first()
                        .filter(|m| m.indices.len() == 2)
                        .map(|m| (m.indices[0], m.indices[1]));
                    fragment_line(&t.fragment, byte_range)
                }
                None => (String::new(), None),
            };
            CodeHit { path: it.path, line, range }
        })
        .collect())
}

/// Extract the single line containing the match (byte indices into the
/// fragment) and convert the match to char indices within that line.
/// GitHub's text-match offsets can land on a line terminator; a match that
/// starts there belongs to the *following* line, so leading `\r`/`\n` bytes
/// are stepped over and the line holding the first matched content wins.
/// Arbitrary offsets (out of range, reversed, mid-UTF-8) are clamped to
/// char boundaries, never panicking.
fn fragment_line(fragment: &str, byte_range: Option<(usize, usize)>) -> (String, Option<(usize, usize)>) {
    let Some((ms, me)) = byte_range else {
        let first = fragment.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
        return (first.trim().to_string(), None);
    };
    let mut ms = ms.min(fragment.len());
    let mut me = me.clamp(ms, fragment.len());
    while ms > 0 && !fragment.is_char_boundary(ms) {
        ms -= 1;
    }
    while me < fragment.len() && !fragment.is_char_boundary(me) {
        me += 1;
    }
    // A match that starts on a line terminator terminates the previous
    // line but belongs to the next one — step to the first content byte.
    while ms < fragment.len() && matches!(fragment.as_bytes()[ms], b'\n' | b'\r') {
        ms += 1;
    }
    let me = me.max(ms);
    let start = fragment[..ms].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = fragment[ms..].find('\n').map(|i| ms + i).unwrap_or(fragment.len());
    let line_raw = &fragment[start..end];
    let trimmed_start = start + (line_raw.len() - line_raw.trim_start().len());
    let line = line_raw.trim_start().trim_end().to_string();
    let rel_s = ms.saturating_sub(trimmed_start).min(line.len());
    let rel_e = me.min(end).saturating_sub(trimmed_start).min(line.len());
    let cs = line[..rel_s].chars().count();
    let ce = line[..rel_e].chars().count();
    (line, Some((cs, ce.max(cs))))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// GitHub returns base64 with embedded newlines.
pub fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    B64.decode(clean.as_bytes()).map_err(|e| format!("base64: {}", e))
}

pub fn b64_encode(data: &[u8]) -> String {
    B64.encode(data)
}

#[cfg(test)]
mod tests {
    use super::fragment_line;

    #[test]
    fn no_range_picks_first_nonempty_line() {
        let (line, range) = fragment_line("\n   \nfn main() {}\n", None);
        assert_eq!(line, "fn main() {}");
        assert_eq!(range, None);
        assert_eq!(fragment_line("", None), (String::new(), None));
    }

    #[test]
    fn match_mid_line() {
        let frag = "alpha\n  beta gamma\ndelta";
        let ms = frag.find("gamma").unwrap();
        let (line, range) = fragment_line(frag, Some((ms, ms + "gamma".len())));
        assert_eq!(line, "beta gamma");
        assert_eq!(range, Some((5, 10)));
    }

    #[test]
    fn match_at_fragment_start_and_end() {
        let frag = "hello\nworld";
        assert_eq!(fragment_line(frag, Some((0, 5))), ("hello".to_string(), Some((0, 5))));
        // Zero-width match at the very end stays on the last line.
        assert_eq!(fragment_line(frag, Some((11, 11))), ("world".to_string(), Some((5, 5))));
    }

    #[test]
    fn match_on_newline_picks_following_line() {
        let frag = "first\nsecond";
        // Match text begins at the '\n' (covers "\nsec").
        let (line, range) = fragment_line(frag, Some((5, 9)));
        assert_eq!(line, "second");
        assert_eq!(range, Some((0, 3)));
        // Match that is exactly the newline maps to the start of the
        // following line.
        let (line, range) = fragment_line(frag, Some((5, 6)));
        assert_eq!(line, "second");
        assert_eq!(range, Some((0, 0)));
    }

    #[test]
    fn match_on_crlf_and_blank_lines_skips_to_content() {
        // "\r\nb" — CRLF terminator stepped over as one unit.
        let (line, range) = fragment_line("a\r\nb", Some((1, 4)));
        assert_eq!(line, "b");
        assert_eq!(range, Some((0, 1)));
        // "\n\nbc" — blank line between match start and content.
        let (line, range) = fragment_line("a\n\nbc", Some((1, 5)));
        assert_eq!(line, "bc");
        assert_eq!(range, Some((0, 2)));
    }

    #[test]
    fn trailing_newline_match_yields_empty_following_line() {
        let (line, range) = fragment_line("abc\n", Some((3, 4)));
        assert_eq!(line, "");
        assert_eq!(range, Some((0, 0)));
    }

    #[test]
    fn multibyte_around_match() {
        let frag = "héllo wörld\nnaïve";
        let ms = frag.find("wörld").unwrap();
        let (line, range) = fragment_line(frag, Some((ms, ms + "wörld".len())));
        assert_eq!(line, "héllo wörld");
        // Char indices, not byte indices.
        assert_eq!(range, Some((6, 11)));
    }

    #[test]
    fn offsets_inside_utf8_sequence_snap_to_boundaries() {
        // Byte 2 sits inside 'é' (bytes 1..3): start snaps down, end up.
        let (line, range) = fragment_line("héllo", Some((2, 2)));
        assert_eq!(line, "héllo");
        assert_eq!(range, Some((1, 2)));
    }

    #[test]
    fn out_of_range_and_reversed_offsets_are_clamped() {
        let frag = "abc\ndef";
        let (line, range) = fragment_line(frag, Some((50, 99)));
        assert_eq!(line, "def");
        assert_eq!(range, Some((3, 3)));
        let (line, range) = fragment_line(frag, Some((2, 1)));
        assert_eq!(line, "abc");
        assert_eq!(range, Some((2, 2)));
        assert_eq!(fragment_line("", Some((3, 7))), (String::new(), Some((0, 0))));
    }

    #[test]
    fn indented_line_range_is_relative_to_trimmed_line() {
        let frag = "fn x() {\n    let y = 1;\n}";
        let ms = frag.find("let").unwrap();
        let (line, range) = fragment_line(frag, Some((ms, ms + 3)));
        assert_eq!(line, "let y = 1;");
        assert_eq!(range, Some((0, 3)));
    }
}

