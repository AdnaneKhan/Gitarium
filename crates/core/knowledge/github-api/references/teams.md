# teams

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/teams — List teams [pg]
  q: team_type(all|enterprise|organization)=all

POST /orgs/{org}/teams — Create a team ->201
  b: name* description maintainers:[] notification_setting(notifications_enabled|notifications_disabled) parent_team_id:i permission(pull|push) privacy(secret|closed) repo_names:[]

GET /orgs/{org}/teams/{team_slug} — Get a team by name

PATCH /orgs/{org}/teams/{team_slug} — Update a team
  b: description name notification_setting(notifications_enabled|notifications_disabled) parent_team_id:i permission(pull|push|admin) privacy(secret|closed)

DELETE /orgs/{org}/teams/{team_slug} — Delete a team ->204

GET /orgs/{org}/teams/{team_slug}/repos — List team repositories [pg]

GET /orgs/{org}/teams/{team_slug}/repos/{owner}/{repo} — Check team permissions for a repository

PUT /orgs/{org}/teams/{team_slug}/repos/{owner}/{repo} — Add or update team repository permissions ->204
  b: permission

DELETE /orgs/{org}/teams/{team_slug}/repos/{owner}/{repo} — Remove a repository from a team ->204

GET /orgs/{org}/teams/{team_slug}/teams — List child teams [pg]

GET /teams/{team_id} — Get a team (Legacy) (deprecated)

PATCH /teams/{team_id} — Update a team (Legacy) (deprecated)
  b: name* description notification_setting(notifications_enabled|notifications_disabled) parent_team_id:i permission(pull|push|admin) privacy(secret|closed)

DELETE /teams/{team_id} — Delete a team (Legacy) ->204 (deprecated)

GET /teams/{team_id}/repos — List team repositories (Legacy) [pg] (deprecated)

GET /teams/{team_id}/repos/{owner}/{repo} — Check team permissions for a repository (Legacy) (deprecated)

PUT /teams/{team_id}/repos/{owner}/{repo} — Add or update team repository permissions (Legacy) ->204 (deprecated)
  b: permission(pull|push|admin)

DELETE /teams/{team_id}/repos/{owner}/{repo} — Remove a repository from a team (Legacy) ->204 (deprecated)

GET /teams/{team_id}/teams — List child teams (Legacy) [pg] (deprecated)

GET /user/teams — List teams for the authenticated user [pg]
