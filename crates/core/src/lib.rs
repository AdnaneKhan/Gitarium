//! RustVM foundation: the runtime substrate shared by every target. No UI,
//! no agent — just an in-memory VFS, HTTP over `globalThis.fetch`, the GitHub
//! REST API, the in-wasm shell (bash/grep/find/jq), and the compiled
//! knowledge bundle. Modules reference each other intra-crate (`crate::fetch`,
//! `crate::vfs`, …), exactly as before the workspace split.

pub mod fetch;
pub mod github;
pub mod knowledge;
pub mod proxy;
pub mod sh;
pub mod store;
pub mod vfs;
