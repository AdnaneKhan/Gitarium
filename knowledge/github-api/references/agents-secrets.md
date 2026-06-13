# agents-secrets

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/agents/secrets — List organization secrets [pg]

GET /orgs/{org}/agents/secrets/public-key — Get an organization public key

GET /orgs/{org}/agents/secrets/{secret_name} — Get an organization secret

PUT /orgs/{org}/agents/secrets/{secret_name} — Create or update an organization secret ->201
  b: encrypted_value* key_id* visibility*(all|private|selected) selected_repository_ids:[i]

DELETE /orgs/{org}/agents/secrets/{secret_name} — Delete an organization secret ->204

GET /orgs/{org}/agents/secrets/{secret_name}/repositories — List selected repositories for an organization secret [pg]

PUT /orgs/{org}/agents/secrets/{secret_name}/repositories — Set selected repositories for an organization secret ->204
  b: selected_repository_ids*:[i]

PUT /orgs/{org}/agents/secrets/{secret_name}/repositories/{repository_id} — Add selected repository to an organization secret ->204

DELETE /orgs/{org}/agents/secrets/{secret_name}/repositories/{repository_id} — Remove selected repository from an organization secret ->204

GET /repos/{owner}/{repo}/agents/organization-secrets — List repository organization secrets [pg]

GET /repos/{owner}/{repo}/agents/secrets — List repository secrets [pg]

GET /repos/{owner}/{repo}/agents/secrets/public-key — Get a repository public key

GET /repos/{owner}/{repo}/agents/secrets/{secret_name} — Get a repository secret

PUT /repos/{owner}/{repo}/agents/secrets/{secret_name} — Create or update a repository secret ->201
  b: encrypted_value* key_id*

DELETE /repos/{owner}/{repo}/agents/secrets/{secret_name} — Delete a repository secret ->204
