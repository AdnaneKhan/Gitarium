# repos-webhooks

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/hooks — List repository webhooks [pg]

POST /repos/{owner}/{repo}/hooks — Create a repository webhook ->201
  b: active:b config{} events:[] name

GET /repos/{owner}/{repo}/hooks/{hook_id} — Get a repository webhook

PATCH /repos/{owner}/{repo}/hooks/{hook_id} — Update a repository webhook
  b: active:b add_events:[] config{} events:[] remove_events:[]

DELETE /repos/{owner}/{repo}/hooks/{hook_id} — Delete a repository webhook ->204

GET /repos/{owner}/{repo}/hooks/{hook_id}/config — Get a webhook configuration for a repository

PATCH /repos/{owner}/{repo}/hooks/{hook_id}/config — Update a webhook configuration for a repository
  b: content_type insecure_ssl secret url

GET /repos/{owner}/{repo}/hooks/{hook_id}/deliveries — List deliveries for a repository webhook [pg]
  q: cursor status(success|failure)

GET /repos/{owner}/{repo}/hooks/{hook_id}/deliveries/{delivery_id} — Get a delivery for a repository webhook

POST /repos/{owner}/{repo}/hooks/{hook_id}/deliveries/{delivery_id}/attempts — Redeliver a delivery for a repository webhook ->202

POST /repos/{owner}/{repo}/hooks/{hook_id}/pings — Ping a repository webhook ->204

POST /repos/{owner}/{repo}/hooks/{hook_id}/tests — Test the push repository webhook ->204
