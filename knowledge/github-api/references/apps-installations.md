# apps-installations

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /installation/repositories — List repositories accessible to the app installation [pg]

DELETE /installation/token — Revoke an installation access token ->204

GET /user/installations — List app installations accessible to the user access token [pg]

GET /user/installations/{installation_id}/repositories — List repositories accessible to the user access token [pg]

PUT /user/installations/{installation_id}/repositories/{repository_id} — Add a repository to an app installation ->204

DELETE /user/installations/{installation_id}/repositories/{repository_id} — Remove a repository from an app installation ->204
