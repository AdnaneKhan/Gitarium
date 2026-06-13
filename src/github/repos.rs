//! Account resolution and the streamed, paginated repo listings.

use serde::Deserialize;

use super::types::{Repo, User};
use super::{api, enc, enc_path, parse};

pub async fn current_user(token: &Option<String>) -> Result<User, String> {
    let (s, b) = api("GET", "/user", token, None).await?;
    parse(s, b)
}

const PER_PAGE: usize = 100;
/// Runaway guard for pagination (10k repos). Anonymous sessions exhaust
/// their 60 req/hour budget long before this; the app stops chaining pages
/// here and flags the list as truncated.
pub const MAX_PAGES: usize = 100;

/// One page of a repo listing. `last` is true when the page came back
/// short, i.e. no further pages exist.
pub struct RepoPage {
    pub repos: Vec<Repo>,
    pub last: bool,
}

/// Fetch one page of `base` (a path that already has a query string, no
/// `page` param). Pages stream to the UI one message at a time, so this
/// returns a single page; the app chains the next request off each result.
pub async fn repos_page(
    token: &Option<String>,
    base: &str,
    page: usize,
) -> Result<RepoPage, String> {
    let (s, b) = api("GET", &format!("{}&page={}", base, page), token, None).await?;
    let repos: Vec<Repo> = parse(s, b)?;
    Ok(RepoPage { last: repos.len() < PER_PAGE, repos })
}

/// Resolve the listing base for a source and fetch its first page.
/// `owner: None` lists the authenticated user's repos; `Some` lists an
/// organization's (falling back to the users endpoint for user accounts).
/// The returned base is what subsequent [`repos_page`] calls paginate.
pub async fn repos_first_page(
    token: &Option<String>,
    owner: Option<&str>,
) -> Result<(String, RepoPage), String> {
    let Some(owner) = owner else {
        let base = format!(
            "/user/repos?per_page={}&sort=pushed&affiliation=owner,collaborator,organization_member",
            PER_PAGE
        );
        let page = repos_page(token, &base, 1).await?;
        return Ok((base, page));
    };
    let org_base =
        format!("/orgs/{}/repos?per_page={}&sort=pushed&type=all", enc(owner), PER_PAGE);
    let (s, b) = api("GET", &format!("{}&page=1", org_base), token, None).await?;
    if s == 404 {
        // The orgs endpoint 404s both for user accounts and for orgs this
        // token cannot see; disambiguate via /users/{owner} (one extra
        // request) so a private org reads as an access error, not a
        // silently shorter public list.
        return match owner_kind(token, owner).await {
            Ok(OwnerKind::User) => {
                let user_base =
                    format!("/users/{}/repos?per_page={}&sort=pushed", enc(owner), PER_PAGE);
                let page = repos_page(token, &user_base, 1).await?;
                Ok((user_base, page))
            }
            Ok(OwnerKind::Organization) => Err(format!(
                "no access to organization '{}' (token may lack org scope or SSO authorization)",
                owner
            )),
            Ok(OwnerKind::Missing) => Err(format!("'{}' not found (or no access)", owner)),
            Err(e) => Err(format!("cannot resolve owner '{}': {}", owner, e)),
        };
    }
    let repos: Vec<Repo> = parse(s, b)?;
    Ok((org_base, RepoPage { last: repos.len() < PER_PAGE, repos }))
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

pub async fn get_repo(token: &Option<String>, full_name: &str) -> Result<Repo, String> {
    let (s, b) = api("GET", &format!("/repos/{}", enc_path(full_name)), token, None).await?;
    parse(s, b)
}
