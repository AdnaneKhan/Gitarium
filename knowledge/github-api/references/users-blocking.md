# users-blocking

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /user/blocks — List users blocked by the authenticated user [pg]

GET /user/blocks/{username} — Check if a user is blocked by the authenticated user ->204

PUT /user/blocks/{username} — Block a user ->204

DELETE /user/blocks/{username} — Unblock a user ->204
