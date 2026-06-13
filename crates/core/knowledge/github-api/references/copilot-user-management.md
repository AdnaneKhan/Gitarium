# copilot-user-management

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/copilot/billing — Get Copilot seat information and settings for an organization

GET /orgs/{org}/copilot/billing/seats — List all Copilot seat assignments for an organization [pg]

POST /orgs/{org}/copilot/billing/selected_teams — Add teams to the Copilot subscription for an organization ->201
  b: selected_teams*:[]

DELETE /orgs/{org}/copilot/billing/selected_teams — Remove teams from the Copilot subscription for an organization
  b: selected_teams*:[]

POST /orgs/{org}/copilot/billing/selected_users — Add users to the Copilot subscription for an organization ->201
  b: selected_usernames*:[]

DELETE /orgs/{org}/copilot/billing/selected_users — Remove users from the Copilot subscription for an organization
  b: selected_usernames*:[]

GET /orgs/{org}/members/{username}/copilot — Get Copilot seat assignment details for a user
