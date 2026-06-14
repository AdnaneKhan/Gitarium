//! Download a repo folder as an in-memory `.tar.gz`. The selected folder's
//! committed blobs are fetched via the git database API, packed with
//! `crate::archive`, and handed to the renderer (which performs the actual
//! browser download — this crate has no DOM). Staged, uncommitted edits are
//! not included: the archive mirrors the current branch's tree.

use crate::{archive, github};

use super::join::join_all;
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
        self.pack(files, base, name);
    }

    /// Download one committed file as a single-entry `.tar.gz`. Same packaging
    /// path as a folder download; the entry name is the file's basename.
    pub(super) fn download_file(&mut self, path: String) {
        let Some(rv) = self.rv.as_ref() else { return };
        let Some(tree) = rv.tree.ready() else {
            self.toast = Some(("tree still loading…".into(), true));
            return;
        };
        let Some(entry) = tree.iter().find(|e| e.kind == "blob" && e.path == path) else {
            self.toast = Some(("file not found in tree".into(), true));
            return;
        };
        let basename = path.rsplit_once('/').map(|(_, n)| n.to_string()).unwrap_or(path);
        let repo = rv.repo.full_name.replace('/', "-");
        self.pack(vec![(basename.clone(), entry.sha.clone())], 0, format!("{}-{}.tar.gz", repo, basename));
    }

    /// Resolve `files`' bytes asynchronously and stage the built `.tar.gz` for
    /// download. `base` strips a parent prefix so the selection is the root.
    fn pack(&mut self, files: Vec<(String, String)>, base: usize, name: String) {
        let Some(rv) = self.rv.as_ref() else { return };
        let token = self.token.clone();
        let full = rv.repo.full_name.clone();
        let n = files.len();
        self.toast = Some((format!("packaging {} file(s)…", n), false));
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
