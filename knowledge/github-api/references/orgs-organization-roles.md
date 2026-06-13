# orgs-organization-roles

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/organization-roles — Get all organization roles for an organization

DELETE /orgs/{org}/organization-roles/teams/{team_slug} — Remove all organization roles for a team ->204

PUT /orgs/{org}/organization-roles/teams/{team_slug}/{role_id} — Assign an organization role to a team ->204

DELETE /orgs/{org}/organization-roles/teams/{team_slug}/{role_id} — Remove an organization role from a team ->204

DELETE /orgs/{org}/organization-roles/users/{username} — Remove all organization roles for a user ->204

PUT /orgs/{org}/organization-roles/users/{username}/{role_id} — Assign an organization role to a user ->204

DELETE /orgs/{org}/organization-roles/users/{username}/{role_id} — Remove an organization role from a user ->204

GET /orgs/{org}/organization-roles/{role_id} — Get an organization role

GET /orgs/{org}/organization-roles/{role_id}/teams — List teams that are assigned to an organization role [pg]

GET /orgs/{org}/organization-roles/{role_id}/users — List users that are assigned to an organization role [pg]
