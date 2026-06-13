# orgs-blocking

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/blocks — List users blocked by an organization [pg]

GET /orgs/{org}/blocks/{username} — Check if a user is blocked by an organization ->204

PUT /orgs/{org}/blocks/{username} — Block a user from an organization ->204

DELETE /orgs/{org}/blocks/{username} — Unblock a user from an organization ->204
