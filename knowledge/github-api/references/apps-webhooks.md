# apps-webhooks

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /app/hook/config — Get a webhook configuration for an app

PATCH /app/hook/config — Update a webhook configuration for an app
  b: content_type insecure_ssl secret url

GET /app/hook/deliveries — List deliveries for an app webhook [pg]
  q: cursor status(success|failure)

GET /app/hook/deliveries/{delivery_id} — Get a delivery for an app webhook

POST /app/hook/deliveries/{delivery_id}/attempts — Redeliver a delivery for an app webhook ->202
