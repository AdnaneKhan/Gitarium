# actions-workflow-jobs

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/actions/jobs/{job_id} — Get a job for a workflow run

GET /repos/{owner}/{repo}/actions/jobs/{job_id}/logs — Download job logs for a workflow run

GET /repos/{owner}/{repo}/actions/runs/{run_id}/attempts/{attempt_number}/jobs — List jobs for a workflow run attempt [pg]

GET /repos/{owner}/{repo}/actions/runs/{run_id}/jobs — List jobs for a workflow run [pg]
  q: filter(latest|all)=latest
