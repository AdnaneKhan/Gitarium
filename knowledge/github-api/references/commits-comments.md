# commits-comments

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/comments — List commit comments for a repository [pg]

GET /repos/{owner}/{repo}/comments/{comment_id} — Get a commit comment

PATCH /repos/{owner}/{repo}/comments/{comment_id} — Update a commit comment
  b: body*

DELETE /repos/{owner}/{repo}/comments/{comment_id} — Delete a commit comment ->204

GET /repos/{owner}/{repo}/commits/{commit_sha}/comments — List commit comments [pg]

POST /repos/{owner}/{repo}/commits/{commit_sha}/comments — Create a commit comment ->201
  b: body* line:i path position:i
