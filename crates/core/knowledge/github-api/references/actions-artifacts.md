# actions-artifacts

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/actions/artifacts — List artifacts for a repository [pg]
  q: name

GET /repos/{owner}/{repo}/actions/artifacts/{artifact_id} — Get an artifact

DELETE /repos/{owner}/{repo}/actions/artifacts/{artifact_id} — Delete an artifact ->204

GET /repos/{owner}/{repo}/actions/artifacts/{artifact_id}/{archive_format} — Download an artifact

GET /repos/{owner}/{repo}/actions/runs/{run_id}/artifacts — List workflow run artifacts [pg]
  q: name direction(asc|desc)=desc
