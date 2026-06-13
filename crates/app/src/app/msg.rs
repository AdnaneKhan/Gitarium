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
    /// Raw logs for a single job (drilled into from the jobs pane).
    JobLogs {
        repo: String,
        job_id: u64,
        result: Result<String, String>,
    },
    /// The 100 most-recently-updated open issues for the Issues tab.
    IssuesLoaded {
        repo: String,
        result: Result<Vec<github::Issue>, String>,
    },
    /// The 100 most-recently-updated open PRs for the Pulls tab.
    PullsLoaded {
        repo: String,
        result: Result<Vec<github::Pull>, String>,
    },
    /// Conversation comments for the open issue/PR detail.
    Comments {
        repo: String,
        number: u64,
        result: Result<Vec<github::Comment>, String>,
    },
    /// The open PR's computed merge state (mergeable / diff stats).
    PullLoaded {
        repo: String,
        number: u64,
        result: Result<github::Pull, String>,
    },
    /// Submitted reviews on the open PR.
    Reviews {
        repo: String,
        number: u64,
        result: Result<Vec<github::Review>, String>,
    },
    /// CI check runs for the open PR's head commit.
    Checks {
        repo: String,
        number: u64,
        result: Result<Vec<github::CheckRun>, String>,
    },
    /// Outcome of an approve (`approve: true`) or merge on the open PR.
    PrActed {
        repo: String,
        number: u64,
        approve: bool,
        result: Result<String, String>,
    },
    CodeSearchDone {
        gen: u64,
        /// 1-based page this result is for: page 1 replaces the list, later
        /// pages append to it.
        page: u32,
        result: Result<(Vec<github::CodeHit>, u64), String>,
    },
    /// The provider's model list for the picker overlay.
    ModelsListed {
        result: Result<Vec<crate::agent::ModelInfo>, String>,
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
            Msg::JobLogs { repo, job_id, result } => self.on_job_logs(repo, job_id, result),
            Msg::IssuesLoaded { repo, result } => self.on_issues_loaded(repo, result),
            Msg::PullsLoaded { repo, result } => self.on_pulls_loaded(repo, result),
            Msg::Comments { repo, number, result } => self.on_comments(repo, number, result),
            Msg::PullLoaded { repo, number, result } => self.on_pull_loaded(repo, number, result),
            Msg::Reviews { repo, number, result } => self.on_reviews(repo, number, result),
            Msg::Checks { repo, number, result } => self.on_checks(repo, number, result),
            Msg::PrActed { repo, number, approve, result } => {
                self.on_pr_acted(repo, number, approve, result)
            }
            Msg::CodeSearchDone { gen, page, result } => self.on_code_search_done(gen, page, result),
            Msg::ModelsListed { result } => self.on_models_listed(result),
            Msg::AgentResponse { gen, result } => self.on_agent_response_msg(gen, result),
            Msg::AgentToolsDone { gen, results } => self.on_agent_tools_done(gen, results),
        }
    }
}
