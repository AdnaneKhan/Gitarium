# codespaces-secrets

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /user/codespaces/secrets — List secrets for the authenticated user [pg]

GET /user/codespaces/secrets/public-key — Get public key for the authenticated user

GET /user/codespaces/secrets/{secret_name} — Get a secret for the authenticated user

PUT /user/codespaces/secrets/{secret_name} — Create or update a secret for the authenticated user ->201
  b: key_id* encrypted_value selected_repository_ids:[]

DELETE /user/codespaces/secrets/{secret_name} — Delete a secret for the authenticated user ->204

GET /user/codespaces/secrets/{secret_name}/repositories — List selected repositories for a user secret

PUT /user/codespaces/secrets/{secret_name}/repositories — Set selected repositories for a user secret ->204
  b: selected_repository_ids*:[i]

PUT /user/codespaces/secrets/{secret_name}/repositories/{repository_id} — Add a selected repository to a user secret ->204

DELETE /user/codespaces/secrets/{secret_name}/repositories/{repository_id} — Remove a selected repository from a user secret ->204
