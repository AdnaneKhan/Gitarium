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
    },
    Branches {
        repo: String,
        result: Result<Vec<github::Branch>, String>,
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
    Committed {
        repo: String,
        branch: String,
        path: String,
        // (new content sha, new commit sha)
        result: Result<(String, String), String>,
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
        repo: String,
        query: String,
        result: Result<Vec<github::CodeHit>, String>,
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
            Msg::ReposPage { gen, base, page, result } => {
                self.on_repos_page(gen, base, page, result)
            }
            Msg::RepoOpened { name, result } => self.on_repo_opened(name, result),
            Msg::Branches { repo, result } => self.on_branches(repo, result),
            Msg::Tree { repo, result } => self.on_tree(repo, result),
            Msg::FileLoaded { repo, branch, path, result } => {
                self.on_file_loaded(repo, branch, path, result)
            }
            Msg::Committed { repo, branch, path, result } => {
                self.on_committed(repo, branch, path, result)
            }
            Msg::Runs { repo, result } => self.on_runs(repo, result),
            Msg::Jobs { repo, run_id, result } => self.on_jobs(repo, run_id, result),
            Msg::CodeSearchDone { repo, query, result } => {
                self.on_code_search_done(repo, query, result)
            }
            Msg::AgentResponse { gen, result } => self.on_agent_response_msg(gen, result),
            Msg::AgentToolsDone { gen, results } => self.on_agent_tools_done(gen, results),
        }
    }
}
