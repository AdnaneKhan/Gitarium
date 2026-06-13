# users-keys

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /user/keys — List public SSH keys for the authenticated user [pg]

POST /user/keys — Create a public SSH key for the authenticated user ->201
  b: key* title

GET /user/keys/{key_id} — Get a public SSH key for the authenticated user

DELETE /user/keys/{key_id} — Delete a public SSH key for the authenticated user ->204

GET /users/{username}/keys — List public keys for a user [pg]
