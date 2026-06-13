# codespaces-organizations

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/codespaces — List codespaces for the organization [pg]

PUT /orgs/{org}/codespaces/access — Manage access control for organization codespaces ->204 (deprecated)
  b: visibility*(disabled|selected_members|all_members|all_members_and_outside_collaborators) selected_usernames:[]

POST /orgs/{org}/codespaces/access/selected_users — Add users to Codespaces access for an organization ->204 (deprecated)
  b: selected_usernames*:[]

DELETE /orgs/{org}/codespaces/access/selected_users — Remove users from Codespaces access for an organization ->204 (deprecated)
  b: selected_usernames*:[]

GET /orgs/{org}/members/{username}/codespaces — List codespaces for a user in organization [pg]

DELETE /orgs/{org}/members/{username}/codespaces/{codespace_name} — Delete a codespace from the organization ->202

POST /orgs/{org}/members/{username}/codespaces/{codespace_name}/stop — Stop a codespace for an organization user
