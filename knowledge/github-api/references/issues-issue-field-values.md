# issues-issue-field-values

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/issues/{issue_number}/issue-field-values — List issue field values for an issue [pg]

PUT /repos/{owner}/{repo}/issues/{issue_number}/issue-field-values — Set issue field values for an issue
  b: issue_field_values:[o]

POST /repos/{owner}/{repo}/issues/{issue_number}/issue-field-values — Add issue field values to an issue
  b: issue_field_values:[o]

DELETE /repos/{owner}/{repo}/issues/{issue_number}/issue-field-values/{issue_field_id} — Delete an issue field value from an issue ->204
