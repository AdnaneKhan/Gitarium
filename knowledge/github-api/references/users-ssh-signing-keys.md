# users-ssh-signing-keys

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /user/ssh_signing_keys — List SSH signing keys for the authenticated user [pg]

POST /user/ssh_signing_keys — Create a SSH signing key for the authenticated user ->201
  b: key* title

GET /user/ssh_signing_keys/{ssh_signing_key_id} — Get an SSH signing key for the authenticated user

DELETE /user/ssh_signing_keys/{ssh_signing_key_id} — Delete an SSH signing key for the authenticated user ->204

GET /users/{username}/ssh_signing_keys — List SSH signing keys for a user [pg]
