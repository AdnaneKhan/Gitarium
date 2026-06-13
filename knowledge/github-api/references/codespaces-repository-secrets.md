# codespaces-repository-secrets

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/codespaces/secrets — List repository secrets [pg]

GET /repos/{owner}/{repo}/codespaces/secrets/public-key — Get a repository public key

GET /repos/{owner}/{repo}/codespaces/secrets/{secret_name} — Get a repository secret

PUT /repos/{owner}/{repo}/codespaces/secrets/{secret_name} — Create or update a repository secret ->201
  b: encrypted_value key_id

DELETE /repos/{owner}/{repo}/codespaces/secrets/{secret_name} — Delete a repository secret ->204
