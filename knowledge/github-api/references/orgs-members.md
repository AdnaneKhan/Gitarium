# orgs-members

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/failed_invitations — List failed organization invitations [pg]

GET /orgs/{org}/invitations — List pending organization invitations [pg]
  q: role(all|admin|direct_member|billing_manager|hiring_manager)=all invitation_source(all|member|scim)=all

POST /orgs/{org}/invitations — Create an organization invitation ->201
  b: email invitee_id:i role(admin|direct_member|billing_manager|reinstate) team_ids:[i]

DELETE /orgs/{org}/invitations/{invitation_id} — Cancel an organization invitation ->204

GET /orgs/{org}/invitations/{invitation_id}/teams — List organization invitation teams [pg]

GET /orgs/{org}/members — List organization members [pg]
  q: filter(2fa_disabled|2fa_insecure|all)=all role(all|admin|member)=all

GET /orgs/{org}/members/{username} — Check organization membership for a user ->204

DELETE /orgs/{org}/members/{username} — Remove an organization member ->204

GET /orgs/{org}/memberships/{username} — Get organization membership for a user

PUT /orgs/{org}/memberships/{username} — Set organization membership for a user
  b: role(admin|member)

DELETE /orgs/{org}/memberships/{username} — Remove organization membership for a user ->204

GET /orgs/{org}/public_members — List public organization members [pg]

GET /orgs/{org}/public_members/{username} — Check public organization membership for a user ->204

PUT /orgs/{org}/public_members/{username} — Set public organization membership for the authenticated user ->204

DELETE /orgs/{org}/public_members/{username} — Remove public organization membership for the authenticated user ->204

GET /user/memberships/orgs — List organization memberships for the authenticated user [pg]
  q: state(active|pending)

GET /user/memberships/orgs/{org} — Get an organization membership for the authenticated user

PATCH /user/memberships/orgs/{org} — Update an organization membership for the authenticated user
  b: state*(active)
