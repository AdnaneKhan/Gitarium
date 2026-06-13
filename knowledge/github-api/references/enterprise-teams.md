# enterprise-teams

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /enterprises/{enterprise}/teams — List enterprise teams [pg]

POST /enterprises/{enterprise}/teams — Create an enterprise team ->201
  b: name* description group_id notification_setting(notifications_enabled|notifications_disabled) organization_selection_type(disabled|selected|all) sync_to_organizations(all|disabled)

GET /enterprises/{enterprise}/teams/{team_slug} — Get an enterprise team

PATCH /enterprises/{enterprise}/teams/{team_slug} — Update an enterprise team
  b: description group_id name notification_setting(notifications_enabled|notifications_disabled) organization_selection_type(disabled|selected|all) sync_to_organizations(all|disabled)

DELETE /enterprises/{enterprise}/teams/{team_slug} — Delete an enterprise team ->204
