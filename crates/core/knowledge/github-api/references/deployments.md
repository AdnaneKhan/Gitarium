# deployments

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/deployments — List deployments [pg]
  q: sha=none ref=none task=none environment=none

POST /repos/{owner}/{repo}/deployments — Create a deployment ->201
  b: ref* auto_merge:b description environment payload production_environment:b required_contexts:[] task transient_environment:b

GET /repos/{owner}/{repo}/deployments/{deployment_id} — Get a deployment

DELETE /repos/{owner}/{repo}/deployments/{deployment_id} — Delete a deployment ->204
