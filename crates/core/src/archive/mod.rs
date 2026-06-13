//! In-memory archive construction: a ustar tar writer plus gzip framing.
//!
//! Built to bundle a folder's blobs — fetched via the git database API and
//! base64-decoded with [`crate::github::b64_decode`] — into a `.tar.gz` the
//! browser can download, sidestepping the `zipball` endpoint (and its audit
//! logging). Everything is pure Rust over byte buffers: no filesystem, no zlib
//! C shim, nothing wasm-hostile.

mod gzip;
mod tar;
#[cfg(test)]
mod tests;

pub use gzip::gzip;
pub use tar::Tar;

/// Build a gzipped tar from `(path, bytes)` entries, in the order given.
///
/// ```ignore
/// let bytes = archive::targz([("src/main.rs", code), ("Cargo.toml", manifest)]);
/// // hand `bytes` to a Blob download as "name.tar.gz".
/// ```
pub fn targz<'a, I>(entries: I) -> Vec<u8>
where
    I: IntoIterator<Item = (&'a str, &'a [u8])>,
{
    let mut tar = Tar::new();
    for (path, data) in entries {
        tar.file(path, data);
    }
    gzip(&tar.finish())
}
