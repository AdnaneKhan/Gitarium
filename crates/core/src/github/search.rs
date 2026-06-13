//! Code search (requires a token; searches default branches only). Two
//! consumers share the fetch: the GUI overlay (repo-scoped, one line +
//! match range per file) and the agent's code_search tool (anywhere on
//! GitHub, several matched lines per file).

use serde::Deserialize;

use super::{api_with_accept, enc, parse};

#[derive(Deserialize)]
struct SearchResp {
    #[serde(default)]
    total_count: u64,
    #[serde(default)]
    incomplete_results: bool,
    #[serde(default)]
    items: Vec<SearchItem>,
}
#[derive(Deserialize)]
struct SearchItem {
    path: String,
    #[serde(default)]
    repository: Option<RepoRef>,
    #[serde(default)]
    text_matches: Vec<TextMatch>,
}
#[derive(Deserialize)]
struct RepoRef {
    #[serde(default)]
    full_name: String,
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

/// Results requested per page — GitHub's Search API maximum. (It also caps
/// the reachable total at 1000, i.e. 10 of these pages.)
pub const SEARCH_PER_PAGE: u32 = 100;

async fn fetch_search(token: &Option<String>, q: &str, page: u32) -> Result<SearchResp, String> {
    let (s, b) = api_with_accept(
        "GET",
        &format!("/search/code?q={}&per_page={}&page={}", enc(q), SEARCH_PER_PAGE, page.max(1)),
        token,
        "application/vnd.github.text-match+json",
        None,
    )
    .await?;
    parse(s, b)
}

/// One code-search hit: the file (with its repo) plus the matched line and
/// the match range in char indices for highlighting.
#[derive(Clone, Debug)]
pub struct CodeHit {
    pub repo: String,
    pub path: String,
    pub line: String,
    pub range: Option<(usize, usize)>,
}

/// One hit per file (matched line + highlight range) for the GUI overlays.
/// `scope` is an optional qualifier appended to the query — `repo:owner/name`
/// for the repo-scoped overlay, `None` for the global overlay (where the
/// user supplies any `org:`/`language:`/… qualifiers themselves). `page` is
/// 1-based; the overlay loads more pages on demand and appends them. Returns
/// the page's hits plus the query's `total_count` (so the caller knows
/// whether further pages exist, up to GitHub's 1000-result cap).
pub async fn search_code(
    token: &Option<String>,
    query: &str,
    scope: Option<&str>,
    page: u32,
) -> Result<(Vec<CodeHit>, u64), String> {
    let q = match scope {
        Some(s) => format!("{} {}", query, s),
        None => query.to_string(),
    };
    let resp = fetch_search(token, &q, page).await?;
    let total = resp.total_count;
    Ok((resp.items.into_iter().map(item_to_hit).collect(), total))
}

fn item_to_hit(it: SearchItem) -> CodeHit {
    let repo = it.repository.map(|r| r.full_name).unwrap_or_default();
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
    CodeHit { repo, path: it.path, line, range }
}

/// One file in an unscoped search: where it lives plus matched lines.
pub struct CodeMatch {
    pub repo: String,
    pub path: String,
    pub lines: Vec<String>,
}

pub struct CodeSearch {
    pub total: u64,
    /// GitHub timed out and returned a partial result set.
    pub incomplete: bool,
    pub items: Vec<CodeMatch>,
}

/// Search anywhere the query's qualifiers allow (agent tool). `page` is
/// 1-based, `SEARCH_PER_PAGE` items per page.
pub async fn search_code_global(
    token: &Option<String>,
    query: &str,
    page: u32,
) -> Result<CodeSearch, String> {
    let resp = fetch_search(token, query, page).await?;
    Ok(CodeSearch {
        total: resp.total_count,
        incomplete: resp.incomplete_results,
        items: resp
            .items
            .into_iter()
            .map(|it| CodeMatch {
                repo: it.repository.map(|r| r.full_name).unwrap_or_default(),
                path: it.path,
                lines: it
                    .text_matches
                    .iter()
                    .filter(|t| !t.fragment.is_empty())
                    .take(3)
                    .map(|t| {
                        let byte_range = t
                            .matches
                            .first()
                            .filter(|m| m.indices.len() == 2)
                            .map(|m| (m.indices[0], m.indices[1]));
                        fragment_line(&t.fragment, byte_range).0
                    })
                    .collect(),
            })
            .collect(),
    })
}

/// Extract the single line containing the match (byte indices into the
/// fragment) and convert the match to char indices within that line.
/// GitHub's text-match offsets can land on a line terminator; a match that
/// starts there belongs to the *following* line, so leading `\r`/`\n` bytes
/// are stepped over and the line holding the first matched content wins.
/// Arbitrary offsets (out of range, reversed, mid-UTF-8) are clamped to
/// char boundaries, never panicking.
pub(super) fn fragment_line(
    fragment: &str,
    byte_range: Option<(usize, usize)>,
) -> (String, Option<(usize, usize)>) {
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
