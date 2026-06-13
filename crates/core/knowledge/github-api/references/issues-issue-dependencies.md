# issues-issue-dependencies

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/issues/{issue_number}/dependencies/blocked_by — List dependencies an issue is blocked by [pg]

POST /repos/{owner}/{repo}/issues/{issue_number}/dependencies/blocked_by — Add a dependency an issue is blocked by ->201
  b: issue_id*:i

DELETE /repos/{owner}/{repo}/issues/{issue_number}/dependencies/blocked_by/{issue_id} — Remove dependency an issue is blocked by

GET /repos/{owner}/{repo}/issues/{issue_number}/dependencies/blocking — List dependencies an issue is blocking [pg]
