# teams-members

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/teams/{team_slug}/invitations — List pending team invitations [pg]

GET /orgs/{org}/teams/{team_slug}/members — List team members [pg]
  q: role(member|maintainer|all)=all

GET /orgs/{org}/teams/{team_slug}/memberships/{username} — Get team membership for a user

PUT /orgs/{org}/teams/{team_slug}/memberships/{username} — Add or update team membership for a user
  b: role(member|maintainer)

DELETE /orgs/{org}/teams/{team_slug}/memberships/{username} — Remove team membership for a user ->204

GET /teams/{team_id}/invitations — List pending team invitations (Legacy) [pg] (deprecated)

GET /teams/{team_id}/members — List team members (Legacy) [pg] (deprecated)
  q: role(member|maintainer|all)=all

GET /teams/{team_id}/members/{username} — Get team member (Legacy) ->204 (deprecated)

PUT /teams/{team_id}/members/{username} — Add team member (Legacy) ->204 (deprecated)

DELETE /teams/{team_id}/members/{username} — Remove team member (Legacy) ->204 (deprecated)

GET /teams/{team_id}/memberships/{username} — Get team membership for a user (Legacy) (deprecated)

PUT /teams/{team_id}/memberships/{username} — Add or update team membership for a user (Legacy) (deprecated)
  b: role(member|maintainer)

DELETE /teams/{team_id}/memberships/{username} — Remove team membership for a user (Legacy) ->204 (deprecated)
