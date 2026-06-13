//! Download a repo folder as an in-memory `.tar.gz`. The selected folder's
//! committed blobs are fetched via the git database API, packed with
//! `crate::archive`, and handed to the renderer (which performs the actual
//! browser download — this crate has no DOM). Staged, uncommitted edits are
//! not included: the archive mirrors the current branch's tree.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{archive, github};

use super::{App, Msg};

impl App {
    /// Pack every committed file under `prefix` (empty = the whole repo) on the
    /// current branch into a `.tar.gz` and stage it for download.
    pub(super) fn download_folder(&mut self, prefix: String) {
        let Some(rv) = self.rv.as_ref() else { return };
        let Some(tree) = rv.tree.ready() else {
            self.toast = Some(("tree still loading…".into(), true));
            return;
        };
        let files: Vec<(String, String)> = tree
            .iter()
            .filter(|e| e.kind == "blob" && under(&e.path, &prefix))
            .map(|e| (e.path.clone(), e.sha.clone()))
            .collect();
        if files.is_empty() {
            self.toast = Some(("nothing to download here".into(), true));
            return;
        }
        // Make the selected folder the archive root by stripping its parent;
        // a whole-repo download keeps full repo-relative paths.
        let base = prefix.rsplit_once('/').map_or(0, |(p, _)| p.len() + 1);
        let name = archive_name(&rv.repo.full_name, &prefix);
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        self.toast = Some((format!("packaging {} file(s)…", files.len()), false));
        crate::spawn_msg(async move {
            Msg::FolderArchive(full.clone(), name, build_targz(&token, &full, &files, base).await)
        });
    }

    pub(super) fn on_folder_archive(&mut self, repo: String, name: String, result: Result<Vec<u8>, String>) {
        // Drop if the user navigated to another repo while it built.
        if self.rv.as_ref().map(|rv| rv.repo.full_name.as_str()) != Some(repo.as_str()) {
            return;
        }
        match result {
            Ok(bytes) => {
                let kb = bytes.len().div_ceil(1024);
                self.toast = Some((format!("{} · {} KB", name, kb), false));
                self.pending_download = Some((name, bytes));
            }
            Err(e) => self.toast = Some((format!("download failed: {}", e), true)),
        }
    }
}

/// Is `path` the folder `prefix` itself or a descendant? Empty prefix = all.
fn under(path: &str, prefix: &str) -> bool {
    prefix.is_empty() || path == prefix || path.strip_prefix(prefix).is_some_and(|r| r.starts_with('/'))
}

/// `owner-repo[-folder].tar.gz`, with path separators flattened to dashes.
fn archive_name(full: &str, prefix: &str) -> String {
    let repo = full.replace('/', "-");
    if prefix.is_empty() {
        format!("{}.tar.gz", repo)
    } else {
        format!("{}-{}.tar.gz", repo, prefix.replace('/', "-"))
    }
}

/// Max REST blob fetches in flight at once — bounded so we pipeline the
/// round-trips without tripping GitHub's secondary (abuse) rate limits.
const FETCH_CONCURRENCY: usize = 8;
/// Blobs requested per GraphQL batch. One request aliases this many
/// `object(oid:)` reads; kept modest to bound the response size.
const GQL_CHUNK: usize = 50;

/// Resolve every file's bytes, then pack them in tree order. Authenticated
/// sessions pull text blobs in bulk over GraphQL (few requests); binary blobs,
/// anonymous sessions, and any GraphQL failure fall through to a byte-exact
/// concurrent REST pass. `base` is stripped so the folder is the archive root.
async fn build_targz(token: &Option<String>, full: &str, files: &[(String, String)], base: usize) -> Result<Vec<u8>, String> {
    let mut contents: Vec<Option<Vec<u8>>> = (0..files.len()).map(|_| None).collect();
    if token.is_some() {
        if let Some((owner, name)) = full.split_once('/') {
            gql_fill(token, owner, name, files, &mut contents).await;
        }
    }
    rest_fill(token, full, files, &mut contents).await?;
    let mut tar = archive::Tar::new();
    for ((path, _), content) in files.iter().zip(contents) {
        tar.file(&path[base..], &content.unwrap_or_default());
    }
    Ok(archive::gzip(&tar.finish()))
}

/// Fill `contents` with UTF-8 text blobs fetched in GraphQL batches. Best
/// effort: anything unresolved (binary, oversized, or a failed request) stays
/// `None` for the REST pass to pick up.
async fn gql_fill(token: &Option<String>, owner: &str, name: &str, files: &[(String, String)], contents: &mut [Option<Vec<u8>>]) {
    for (ci, chunk) in files.chunks(GQL_CHUNK).enumerate() {
        let oids: Vec<&str> = chunk.iter().map(|(_, sha)| sha.as_str()).collect();
        let Ok(texts) = github::blob_texts(token, owner, name, &oids).await else { continue };
        for (j, text) in texts.into_iter().enumerate() {
            if let Some(t) = text {
                contents[ci * GQL_CHUNK + j] = Some(t.into_bytes());
            }
        }
    }
}

/// Fetch every still-unresolved blob over REST, bounded-concurrently and
/// byte-exact (base64). Errors abort the whole download.
async fn rest_fill(token: &Option<String>, full: &str, files: &[(String, String)], contents: &mut [Option<Vec<u8>>]) -> Result<(), String> {
    let todo: Vec<usize> = (0..files.len()).filter(|&i| contents[i].is_none()).collect();
    for chunk in todo.chunks(FETCH_CONCURRENCY) {
        let fetched = join_all(chunk.iter().map(|&i| async move {
            let blob = github::get_blob(token, full, &files[i].1).await?;
            Ok::<(usize, Vec<u8>), String>((i, github::b64_decode(&blob.content)?))
        }))
        .await;
        for entry in fetched {
            let (i, bytes) = entry?;
            contents[i] = Some(bytes);
        }
    }
    Ok(())
}

/// Drive a set of futures concurrently to completion, returning their outputs
/// in input order. Minimal stand-in for `futures::future::join_all` (no such
/// dependency here); each poll advances every still-pending child. Bound the
/// input size by the caller — this polls all entries it's given.
fn join_all<F: Future>(futs: impl IntoIterator<Item = F>) -> JoinAll<F> {
    let futs: Vec<_> = futs.into_iter().map(|f| Some(Box::pin(f))).collect();
    let out = futs.iter().map(|_| None).collect();
    JoinAll { futs, out }
}

struct JoinAll<F: Future> {
    futs: Vec<Option<Pin<Box<F>>>>,
    out: Vec<Option<F::Output>>,
}

// The futures are heap-pinned in `Box`es (never moved) and only finished
// outputs are moved out, so JoinAll is safe to treat as Unpin for any `F`.
impl<F: Future> Unpin for JoinAll<F> {}

impl<F: Future> Future for JoinAll<F> {
    type Output = Vec<F::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.get_mut();
        let mut pending = false;
        for (slot, out) in me.futs.iter_mut().zip(me.out.iter_mut()) {
            if let Some(fut) = slot {
                match fut.as_mut().poll(cx) {
                    Poll::Ready(v) => {
                        *out = Some(v);
                        *slot = None;
                    }
                    Poll::Pending => pending = true,
                }
            }
        }
        if pending {
            Poll::Pending
        } else {
            Poll::Ready(me.out.iter_mut().map(|o| o.take().unwrap()).collect())
        }
    }
}
