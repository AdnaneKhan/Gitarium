# deployments-statuses

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/deployments/{deployment_id}/statuses — List deployment statuses [pg]

POST /repos/{owner}/{repo}/deployments/{deployment_id}/statuses — Create a deployment status ->201
  b: state*(error|failure|inactive|in_progress|queued|…) auto_inactive:b description environment environment_url log_url target_url

GET /repos/{owner}/{repo}/deployments/{deployment_id}/statuses/{status_id} — Get a deployment status
