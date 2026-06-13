# actions-concurrency-groups

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/actions/concurrency_groups — List concurrency groups for a repository [pg]
  q: after

GET /repos/{owner}/{repo}/actions/concurrency_groups/{concurrency_group_name} — Get a concurrency group for a repository
  q: ahead_of_run:i ahead_of_job:i

GET /repos/{owner}/{repo}/actions/runs/{run_id}/concurrency_groups — List concurrency groups for a workflow run [pg]
  q: before after
