//! Workflow runs and their jobs (the Actions tab).

use super::types::{Job, JobsResp, Run, RunsResp};
use super::{api, enc_path, parse};

pub async fn list_runs(token: &Option<String>, full_name: &str) -> Result<Vec<Run>, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/actions/runs?per_page=50", enc_path(full_name)),
        token,
        None,
    )
    .await?;
    let r: RunsResp = parse(s, b)?;
    Ok(r.workflow_runs)
}

pub async fn list_jobs(token: &Option<String>, full_name: &str, run_id: u64) -> Result<Vec<Job>, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/actions/runs/{}/jobs?per_page=100", enc_path(full_name), run_id),
        token,
        None,
    )
    .await?;
    let r: JobsResp = parse(s, b)?;
    Ok(r.jobs)
}
