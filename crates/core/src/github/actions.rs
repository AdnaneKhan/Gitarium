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

/// Raw plain-text logs for one job. The endpoint 302-redirects to a signed
/// blob URL; `fetch` follows it and returns the text. Requires auth, and the
/// blob host may lack CORS in a direct browser — callers surface the error
/// (with an "open on GitHub" fallback) when that happens.
pub async fn get_job_logs(token: &Option<String>, full_name: &str, job_id: u64) -> Result<String, String> {
    let (s, b) = api(
        "GET",
        &format!("/repos/{}/actions/jobs/{}/logs", enc_path(full_name), job_id),
        token,
        None,
    )
    .await?;
    if !(200..300).contains(&s) {
        let msg: String = b.chars().take(160).collect();
        return Err(format!("HTTP {}: {}", s, msg));
    }
    Ok(b)
}

/// Delete a workflow run. Requires write access (the Actions `repo` scope);
/// GitHub returns 204 No Content with an empty body, so — like the logs
/// endpoint — we check the status range rather than parse JSON.
pub async fn delete_workflow_run(
    token: &Option<String>,
    full_name: &str,
    run_id: u64,
) -> Result<(), String> {
    let (s, b) = api(
        "DELETE",
        &format!("/repos/{}/actions/runs/{}", enc_path(full_name), run_id),
        token,
        None,
    )
    .await?;
    if !(200..300).contains(&s) {
        let msg: String = b.chars().take(160).collect();
        return Err(format!("HTTP {}: {}", s, msg));
    }
    Ok(())
}
