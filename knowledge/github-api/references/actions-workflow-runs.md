# actions-workflow-runs

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

POST /repos/{owner}/{repo}/actions/jobs/{job_id}/rerun — Re-run a job from a workflow run ->201
  b: enable_debug_logging:b enable_debugger:b

GET /repos/{owner}/{repo}/actions/runs — List workflow runs for a repository [pg]
  q: actor branch event status(completed|action_required|cancelled|failure|neutral|…) created exclude_pull_requests:b=false check_suite_id:i head_sha

GET /repos/{owner}/{repo}/actions/runs/{run_id} — Get a workflow run
  q: exclude_pull_requests:b=false

DELETE /repos/{owner}/{repo}/actions/runs/{run_id} — Delete a workflow run ->204

GET /repos/{owner}/{repo}/actions/runs/{run_id}/approvals — Get the review history for a workflow run

POST /repos/{owner}/{repo}/actions/runs/{run_id}/approve — Approve a workflow run for a fork pull request ->201

GET /repos/{owner}/{repo}/actions/runs/{run_id}/attempts/{attempt_number} — Get a workflow run attempt
  q: exclude_pull_requests:b=false

GET /repos/{owner}/{repo}/actions/runs/{run_id}/attempts/{attempt_number}/logs — Download workflow run attempt logs

POST /repos/{owner}/{repo}/actions/runs/{run_id}/cancel — Cancel a workflow run ->202

POST /repos/{owner}/{repo}/actions/runs/{run_id}/deployment_protection_rule — Review custom deployment protection rules for a workflow run ->204
  b: comment environment_name state(approved|rejected) (one-of)

POST /repos/{owner}/{repo}/actions/runs/{run_id}/force-cancel — Force cancel a workflow run ->202

GET /repos/{owner}/{repo}/actions/runs/{run_id}/logs — Download workflow run logs

DELETE /repos/{owner}/{repo}/actions/runs/{run_id}/logs — Delete workflow run logs ->204

GET /repos/{owner}/{repo}/actions/runs/{run_id}/pending_deployments — Get pending deployments for a workflow run

POST /repos/{owner}/{repo}/actions/runs/{run_id}/pending_deployments — Review pending deployments for a workflow run
  b: comment* environment_ids*:[i] state*(approved|rejected)

POST /repos/{owner}/{repo}/actions/runs/{run_id}/rerun — Re-run a workflow ->201
  b: enable_debug_logging:b

POST /repos/{owner}/{repo}/actions/runs/{run_id}/rerun-failed-jobs — Re-run failed jobs from a workflow run ->201
  b: enable_debug_logging:b

GET /repos/{owner}/{repo}/actions/runs/{run_id}/timing — Get workflow run usage

GET /repos/{owner}/{repo}/actions/workflows/{workflow_id}/runs — List workflow runs for a workflow [pg]
  q: actor branch event status(completed|action_required|cancelled|failure|neutral|…) created exclude_pull_requests:b=false check_suite_id:i head_sha
