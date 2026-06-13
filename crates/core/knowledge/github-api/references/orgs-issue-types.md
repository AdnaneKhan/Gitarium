# orgs-issue-types

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/issue-types — List issue types for an organization

POST /orgs/{org}/issue-types — Create issue type for an organization
  b: is_enabled*:b name* color(gray|blue|green|yellow|orange|…) description

PUT /orgs/{org}/issue-types/{issue_type_id} — Update issue type for an organization
  b: is_enabled*:b name* color(gray|blue|green|yellow|orange|…) description

DELETE /orgs/{org}/issue-types/{issue_type_id} — Delete issue type for an organization ->204
