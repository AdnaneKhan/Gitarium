# actions-workflows

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/actions/workflows — List repository workflows [pg]

GET /repos/{owner}/{repo}/actions/workflows/{workflow_id} — Get a workflow

PUT /repos/{owner}/{repo}/actions/workflows/{workflow_id}/disable — Disable a workflow ->204

POST /repos/{owner}/{repo}/actions/workflows/{workflow_id}/dispatches — Create a workflow dispatch event
  b: ref* inputs{} return_run_details:b

PUT /repos/{owner}/{repo}/actions/workflows/{workflow_id}/enable — Enable a workflow ->204

GET /repos/{owner}/{repo}/actions/workflows/{workflow_id}/timing — Get workflow usage
