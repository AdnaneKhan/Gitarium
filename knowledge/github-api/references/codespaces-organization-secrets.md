# codespaces-organization-secrets

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/codespaces/secrets — List organization secrets [pg]

GET /orgs/{org}/codespaces/secrets/public-key — Get an organization public key

GET /orgs/{org}/codespaces/secrets/{secret_name} — Get an organization secret

PUT /orgs/{org}/codespaces/secrets/{secret_name} — Create or update an organization secret ->201
  b: visibility*(all|private|selected) encrypted_value key_id selected_repository_ids:[i]

DELETE /orgs/{org}/codespaces/secrets/{secret_name} — Delete an organization secret ->204

GET /orgs/{org}/codespaces/secrets/{secret_name}/repositories — List selected repositories for an organization secret [pg]

PUT /orgs/{org}/codespaces/secrets/{secret_name}/repositories — Set selected repositories for an organization secret ->204
  b: selected_repository_ids*:[i]

PUT /orgs/{org}/codespaces/secrets/{secret_name}/repositories/{repository_id} — Add selected repository to an organization secret ->204

DELETE /orgs/{org}/codespaces/secrets/{secret_name}/repositories/{repository_id} — Remove selected repository from an organization secret ->204
