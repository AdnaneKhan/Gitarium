# orgs-issue-fields

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/issue-fields — List issue fields for an organization

POST /orgs/{org}/issue-fields — Create issue field for an organization
  b: data_type*(text|date|single_select|multi_select|number) name* description options:[o] visibility(organization_members_only|all)

PATCH /orgs/{org}/issue-fields/{issue_field_id} — Update issue field for an organization
  b: description name options:[o] visibility(organization_members_only|all)

DELETE /orgs/{org}/issue-fields/{issue_field_id} — Delete issue field for an organization ->204
