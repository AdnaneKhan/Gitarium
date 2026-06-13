# enterprise-teams-enterprise-team-members

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /enterprises/{enterprise}/teams/{enterprise-team}/memberships — List members in an enterprise team [pg]

POST /enterprises/{enterprise}/teams/{enterprise-team}/memberships/add — Bulk add team members
  b: usernames*:[]

POST /enterprises/{enterprise}/teams/{enterprise-team}/memberships/remove — Bulk remove team members
  b: usernames*:[]

GET /enterprises/{enterprise}/teams/{enterprise-team}/memberships/{username} — Get enterprise team membership

PUT /enterprises/{enterprise}/teams/{enterprise-team}/memberships/{username} — Add team member ->201

DELETE /enterprises/{enterprise}/teams/{enterprise-team}/memberships/{username} — Remove team membership ->204
