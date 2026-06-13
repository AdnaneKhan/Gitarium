# campaigns

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/campaigns — List campaigns for an organization [pg]
  q: direction(asc|desc)=desc state(open|closed) sort(created|updated|ends_at|published)=created

POST /orgs/{org}/campaigns — Create a campaign for an organization
  b: description* ends_at* name* code_scanning_alerts:[o] contact_link generate_issues:b managers:[] team_managers:[]

GET /orgs/{org}/campaigns/{campaign_number} — Get a campaign for an organization

PATCH /orgs/{org}/campaigns/{campaign_number} — Update a campaign
  b: contact_link description ends_at managers:[] name state(open|closed) team_managers:[]

DELETE /orgs/{org}/campaigns/{campaign_number} — Delete a campaign for an organization ->204
