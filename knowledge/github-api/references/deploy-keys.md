# deploy-keys

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/keys — List deploy keys [pg]

POST /repos/{owner}/{repo}/keys — Create a deploy key ->201
  b: key* read_only:b title

GET /repos/{owner}/{repo}/keys/{key_id} — Get a deploy key

DELETE /repos/{owner}/{repo}/keys/{key_id} — Delete a deploy key ->204
