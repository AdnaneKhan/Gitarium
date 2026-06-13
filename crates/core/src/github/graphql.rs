//! Batched blob reads over the GitHub GraphQL API. One request pulls many
//! blobs' UTF-8 text via aliased `object(oid:)` fields, collapsing the N
//! round-trips the REST blobs endpoint would need. Binary blobs (and ones too
//! large for GraphQL to inline) come back as `None` — the caller fetches those
//! byte-exact over REST. GraphQL requires a token, so callers gate on auth.

use serde_json::Value;

use super::api;

/// Fetch the UTF-8 text of each blob `oid` in a single GraphQL request.
/// Returns one entry per input oid, in order: `Some(text)` for a text blob,
/// `None` for a binary/oversized blob (no inline text — fetch via REST).
///
/// `owner`/`repo` come from a GitHub `full_name`, whose charset can't contain
/// quotes, and `oid`s are hex — so interpolating them into the query is safe.
pub async fn blob_texts(token: &Option<String>, owner: &str, repo: &str, oids: &[&str]) -> Result<Vec<Option<String>>, String> {
    let mut sel = String::new();
    for (i, oid) in oids.iter().enumerate() {
        sel.push_str(&format!("b{}: object(oid: \"{}\") {{ ... on Blob {{ text isBinary }} }} ", i, oid));
    }
    let query = format!("{{ repository(owner: \"{}\", name: \"{}\") {{ {} }} }}", owner, repo, sel);
    let body = serde_json::json!({ "query": query }).to_string();
    let (status, text) = api("POST", "/graphql", token, Some(body)).await?;
    if !(200..300).contains(&status) {
        return Err(format!("graphql HTTP {}", status));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("graphql parse: {}", e))?;
    if let Some(e) = v.get("errors").and_then(Value::as_array).and_then(|a| a.first()) {
        return Err(e.get("message").and_then(Value::as_str).unwrap_or("graphql error").to_string());
    }
    let repo_obj = v
        .get("data")
        .and_then(|d| d.get("repository"))
        .ok_or_else(|| "graphql: missing repository".to_string())?;
    let out = (0..oids.len())
        .map(|i| {
            let b = repo_obj.get(format!("b{}", i).as_str());
            let binary = b.and_then(|b| b.get("isBinary")).and_then(Value::as_bool).unwrap_or(false);
            match b.and_then(|b| b.get("text")).and_then(Value::as_str) {
                Some(t) if !binary => Some(t.to_string()),
                _ => None,
            }
        })
        .collect();
    Ok(out)
}
