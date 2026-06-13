# issues-comments

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/issues/comments — List issue comments for a repository [pg]
  q: sort(created|updated)=created direction(asc|desc) since

GET /repos/{owner}/{repo}/issues/comments/{comment_id} — Get an issue comment

PATCH /repos/{owner}/{repo}/issues/comments/{comment_id} — Update an issue comment
  b: body*

DELETE /repos/{owner}/{repo}/issues/comments/{comment_id} — Delete an issue comment ->204

PUT /repos/{owner}/{repo}/issues/comments/{comment_id}/pin — Pin an issue comment

DELETE /repos/{owner}/{repo}/issues/comments/{comment_id}/pin — Unpin an issue comment ->204

GET /repos/{owner}/{repo}/issues/{issue_number}/comments — List issue comments [pg]
  q: since

POST /repos/{owner}/{repo}/issues/{issue_number}/comments — Create an issue comment ->201
  b: body*
