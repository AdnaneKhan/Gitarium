# orgs-outside-collaborators

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/outside_collaborators — List outside collaborators for an organization [pg]
  q: filter(2fa_disabled|2fa_insecure|all)=all

PUT /orgs/{org}/outside_collaborators/{username} — Convert an organization member to outside collaborator ->202
  b: async:b

DELETE /orgs/{org}/outside_collaborators/{username} — Remove outside collaborator from an organization ->204
