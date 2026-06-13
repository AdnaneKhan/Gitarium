# users-gpg-keys

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /user/gpg_keys — List GPG keys for the authenticated user [pg]

POST /user/gpg_keys — Create a GPG key for the authenticated user ->201
  b: armored_public_key* name

GET /user/gpg_keys/{gpg_key_id} — Get a GPG key for the authenticated user

DELETE /user/gpg_keys/{gpg_key_id} — Delete a GPG key for the authenticated user ->204

GET /users/{username}/gpg_keys — List GPG keys for a user [pg]
