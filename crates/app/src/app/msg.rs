//! Async results delivered to the app, one enum variant per request kind,
//! each carrying enough context to detect staleness. `on_msg` only
//! dispatches; the handlers live with their topic modules.

use crate::github;

use super::App;

pub enum Msg {
    TokenChecked {
        token: Option<String>,
        result: Result<github::User, String>,
    },
    /// Result of the proxy-mode "log in with the server's token" attempt
    /// (empty token field while the API proxy is on). Ok → adopt the server
    /// identity; Err → fall back to anonymous, silently (no token configured).
    ProxyAuthChecked {
        result: Result<github::User, String>,
    },
    /// One page of the repo listing; the view populates as pages land and
    /// the handler chains the next request until a short page or the cap.
    ReposPage {
        gen: u64,
        /// Resolved listing base (org vs user endpoint), echoed back so the
        /// next page hits the same endpoint without re-resolving.
        base: String,
        page: usize,
        result: Result<github::RepoPage, String>,
    },
    RepoOpened {
        name: String,
        result: Result<github::Repo, String>,
        /// File path to open once the repo is loaded (global code-search
        /// hit); None for a plain repo open.
        then_open: Option<String>,
    },
    Branches {
        repo: String,
        result: Result<Vec<github::Branch>, String>,
    },
    /// A new branch ref was created from the branch picker; `sha` is the base
    /// head it points at, so the view can switch to it without a reload.
    BranchCreated {
        repo: String,
        name: String,
        sha: String,
        result: Result<(), String>,
    },
    Tree {
        repo: String,
        result: Result<github::TreeResp, String>,
    },
    FileLoaded {
        repo: String,
        branch: String,
        path: String,
        result: Result<(String, Vec<u8>), String>,
    },
    /// A staged Git DB commit finished: the new commit sha, or an error.
    /// `name` is the branch (current or new) or tag the commit landed on;
    /// `is_tag` distinguishes the two.
    Committed {
        repo: String,
        name: String,
        is_tag: bool,
        result: Result<String, String>,
    },
    Runs {
        repo: String,
        result: Result<Vec<github::Run>, String>,
    },
    Jobs {
        repo: String,
        run_id: u64,
        result: Result<Vec<github::Job>, String>,
    },
    CodeSearchDone {
        gen: u64,
        /// 1-based page this result is for: page 1 replaces the list, later
        /// pages append to it.
        page: u32,
        result: Result<(Vec<github::CodeHit>, u64), String>,
    },
    /// One Messages API response in the agent loop.
    AgentResponse {
        gen: u64,
        result: Result<serde_json::Value, String>,
    },
    /// All tool calls of one turn finished: (tool_result block, ok) each.
    AgentToolsDone {
        gen: u64,
        results: Vec<(serde_json::Value, bool)>,
    },
}

impl App {
    pub fn on_msg(&mut self, msg: Msg) {
        self.dirty = true;
        match msg {
            Msg::TokenChecked { token, result } => self.on_token_checked(token, result),
            Msg::ProxyAuthChecked { result } => self.on_proxy_auth_checked(result),
            Msg::ReposPage { gen, base, page, result } => {
                self.on_repos_page(gen, base, page, result)
            }
            Msg::RepoOpened { name, result, then_open } => {
                self.on_repo_opened(name, result, then_open)
            }
            Msg::Branches { repo, result } => self.on_branches(repo, result),
            Msg::BranchCreated { repo, name, sha, result } => {
                self.on_branch_created(repo, name, sha, result)
            }
            Msg::Tree { repo, result } => self.on_tree(repo, result),
            Msg::FileLoaded { repo, branch, path, result } => {
                self.on_file_loaded(repo, branch, path, result)
            }
            Msg::Committed { repo, name, is_tag, result } => {
                self.on_committed(repo, name, is_tag, result)
            }
            Msg::Runs { repo, result } => self.on_runs(repo, result),
            Msg::Jobs { repo, run_id, result } => self.on_jobs(repo, run_id, result),
            Msg::CodeSearchDone { gen, page, result } => self.on_code_search_done(gen, page, result),
            Msg::AgentResponse { gen, result } => self.on_agent_response_msg(gen, result),
            Msg::AgentToolsDone { gen, results } => self.on_agent_tools_done(gen, results),
        }
    }
}
