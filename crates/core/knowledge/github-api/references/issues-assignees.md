# issues-assignees

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/assignees — List assignees [pg]

GET /repos/{owner}/{repo}/assignees/{assignee} — Check if a user can be assigned ->204

POST /repos/{owner}/{repo}/issues/{issue_number}/assignees — Add assignees to an issue ->201
  b: assignees:[]

DELETE /repos/{owner}/{repo}/issues/{issue_number}/assignees — Remove assignees from an issue
  b: assignees:[]

GET /repos/{owner}/{repo}/issues/{issue_number}/assignees/{assignee} — Check if a user can be assigned to a issue ->204
