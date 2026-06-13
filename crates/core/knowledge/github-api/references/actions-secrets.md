# actions-secrets

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/actions/secrets — List organization secrets [pg]

GET /orgs/{org}/actions/secrets/public-key — Get an organization public key

GET /orgs/{org}/actions/secrets/{secret_name} — Get an organization secret

PUT /orgs/{org}/actions/secrets/{secret_name} — Create or update an organization secret ->201
  b: encrypted_value* key_id* visibility*(all|private|selected) selected_repository_ids:[i]

DELETE /orgs/{org}/actions/secrets/{secret_name} — Delete an organization secret ->204

GET /orgs/{org}/actions/secrets/{secret_name}/repositories — List selected repositories for an organization secret [pg]

PUT /orgs/{org}/actions/secrets/{secret_name}/repositories — Set selected repositories for an organization secret ->204
  b: selected_repository_ids*:[i]

PUT /orgs/{org}/actions/secrets/{secret_name}/repositories/{repository_id} — Add selected repository to an organization secret ->204

DELETE /orgs/{org}/actions/secrets/{secret_name}/repositories/{repository_id} — Remove selected repository from an organization secret ->204

GET /repos/{owner}/{repo}/actions/organization-secrets — List repository organization secrets [pg]

GET /repos/{owner}/{repo}/actions/secrets — List repository secrets [pg]

GET /repos/{owner}/{repo}/actions/secrets/public-key — Get a repository public key

GET /repos/{owner}/{repo}/actions/secrets/{secret_name} — Get a repository secret

PUT /repos/{owner}/{repo}/actions/secrets/{secret_name} — Create or update a repository secret ->201
  b: encrypted_value* key_id*

DELETE /repos/{owner}/{repo}/actions/secrets/{secret_name} — Delete a repository secret ->204

GET /repos/{owner}/{repo}/environments/{environment_name}/secrets — List environment secrets [pg]

GET /repos/{owner}/{repo}/environments/{environment_name}/secrets/public-key — Get an environment public key

GET /repos/{owner}/{repo}/environments/{environment_name}/secrets/{secret_name} — Get an environment secret

PUT /repos/{owner}/{repo}/environments/{environment_name}/secrets/{secret_name} — Create or update an environment secret ->201
  b: encrypted_value* key_id*

DELETE /repos/{owner}/{repo}/environments/{environment_name}/secrets/{secret_name} — Delete an environment secret ->204
