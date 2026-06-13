# apps-oauth-applications

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

DELETE /applications/{client_id}/grant — Delete an app authorization ->204
  b: access_token*

POST /applications/{client_id}/token — Check a token
  b: access_token*

PATCH /applications/{client_id}/token — Reset a token
  b: access_token*

DELETE /applications/{client_id}/token — Delete an app token ->204
  b: access_token*
