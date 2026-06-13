//! The Claude-powered GitHub agent: the Messages API loop, tool set
//! (github_api, code_search, bash/grep/find), context compaction, and the
//! UI-free `headless` driver. Depends only on the foundation, so it links
//! into headless-only targets with no rendering code.
//!
//! Re-importing the foundation modules under their old names keeps the
//! agent's `crate::github` / `crate::sh` / … paths working unchanged.
use gitarium_core::{fetch, github, knowledge, proxy, sh, vfs};

pub mod agent;
