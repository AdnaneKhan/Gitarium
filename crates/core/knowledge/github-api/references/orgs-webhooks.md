# orgs-webhooks

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/hooks — List organization webhooks [pg]

POST /orgs/{org}/hooks — Create an organization webhook ->201
  b: config*{url*} name* active:b events:[]

GET /orgs/{org}/hooks/{hook_id} — Get an organization webhook

PATCH /orgs/{org}/hooks/{hook_id} — Update an organization webhook
  b: active:b config{url*} events:[] name

DELETE /orgs/{org}/hooks/{hook_id} — Delete an organization webhook ->204

GET /orgs/{org}/hooks/{hook_id}/config — Get a webhook configuration for an organization

PATCH /orgs/{org}/hooks/{hook_id}/config — Update a webhook configuration for an organization
  b: content_type insecure_ssl secret url

GET /orgs/{org}/hooks/{hook_id}/deliveries — List deliveries for an organization webhook [pg]
  q: cursor status(success|failure)

GET /orgs/{org}/hooks/{hook_id}/deliveries/{delivery_id} — Get a webhook delivery for an organization webhook

POST /orgs/{org}/hooks/{hook_id}/deliveries/{delivery_id}/attempts — Redeliver a delivery for an organization webhook ->202

POST /orgs/{org}/hooks/{hook_id}/pings — Ping an organization webhook ->204
