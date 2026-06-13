# apps

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /app — Get the authenticated app

POST /app-manifests/{code}/conversions — Create a GitHub App from a manifest ->201

GET /app/installation-requests — List installation requests for the authenticated app [pg]

GET /app/installations — List installations for the authenticated app [pg]
  q: since outdated

GET /app/installations/{installation_id} — Get an installation for the authenticated app

DELETE /app/installations/{installation_id} — Delete an installation for the authenticated app ->204

POST /app/installations/{installation_id}/access_tokens — Create an installation access token for an app ->201
  b: permissions{} repositories:[] repository_ids:[i]

PUT /app/installations/{installation_id}/suspended — Suspend an app installation ->204

DELETE /app/installations/{installation_id}/suspended — Unsuspend an app installation ->204

POST /applications/{client_id}/token/scoped — Create a scoped access token
  b: access_token* permissions{} repositories:[] repository_ids:[i] target target_id:i

GET /apps/{app_slug} — Get an app

GET /orgs/{org}/installation — Get an organization installation for the authenticated app

GET /repos/{owner}/{repo}/installation — Get a repository installation for the authenticated app

GET /users/{username}/installation — Get a user installation for the authenticated app
