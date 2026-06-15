//! The async-result dispatch: routes each drained [`Msg`] to its handler.
//! Split from `msg.rs` (which now holds only the `Msg` data type) so neither
//! file outgrows the line cap.

use super::{App, Msg};

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
            Msg::Branches { repo, page, result } => self.on_branches(repo, page, result),
            Msg::DefaultBranch { repo, result } => self.on_default_branch(repo, result),
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
            Msg::SettingsLoaded { repo, section, result } => {
                self.on_settings_loaded(repo, section, result)
            }
            Msg::SettingsMutated { repo, section, result } => {
                self.on_settings_mutated(repo, section, result)
            }
            Msg::RepoMetaUpdated { repo, result } => self.on_repo_meta_updated(repo, result),
            Msg::RepoDeleted { repo, result } => self.on_repo_deleted(repo, result),
            Msg::RunDeleted { repo, run_id, result } => self.on_run_deleted(repo, run_id, result),
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
            Msg::FolderArchive(repo, name, result) => self.on_folder_archive(repo, name, result),
        }
    }
}
