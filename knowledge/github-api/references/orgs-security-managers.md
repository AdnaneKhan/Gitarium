# orgs-security-managers

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/security-managers — List security manager teams (deprecated)

PUT /orgs/{org}/security-managers/teams/{team_slug} — Add a security manager team ->204 (deprecated)

DELETE /orgs/{org}/security-managers/teams/{team_slug} — Remove a security manager team ->204 (deprecated)
