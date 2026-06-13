# enterprise-teams-enterprise-team-organizations

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /enterprises/{enterprise}/teams/{enterprise-team}/organizations — Get organization assignments [pg]

POST /enterprises/{enterprise}/teams/{enterprise-team}/organizations/add — Add organization assignments
  b: organization_slugs*:[]

POST /enterprises/{enterprise}/teams/{enterprise-team}/organizations/remove — Remove organization assignments ->204
  b: organization_slugs*:[]

GET /enterprises/{enterprise}/teams/{enterprise-team}/organizations/{org} — Get organization assignment

PUT /enterprises/{enterprise}/teams/{enterprise-team}/organizations/{org} — Add an organization assignment ->201

DELETE /enterprises/{enterprise}/teams/{enterprise-team}/organizations/{org} — Delete an organization assignment ->204
