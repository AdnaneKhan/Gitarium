# pulls-comments

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/pulls/comments — List review comments in a repository [pg]
  q: sort(created|updated|created_at) direction(asc|desc) since

GET /repos/{owner}/{repo}/pulls/comments/{comment_id} — Get a review comment for a pull request

PATCH /repos/{owner}/{repo}/pulls/comments/{comment_id} — Update a review comment for a pull request
  b: body*

DELETE /repos/{owner}/{repo}/pulls/comments/{comment_id} — Delete a review comment for a pull request ->204

GET /repos/{owner}/{repo}/pulls/{pull_number}/comments — List review comments on a pull request [pg]
  q: sort(created|updated)=created direction(asc|desc) since

POST /repos/{owner}/{repo}/pulls/{pull_number}/comments — Create a review comment for a pull request ->201
  b: body* commit_id* path* in_reply_to:i line:i position:i side(LEFT|RIGHT) start_line:i start_side(LEFT|RIGHT|side) subject_type(line|file)

POST /repos/{owner}/{repo}/pulls/{pull_number}/comments/{comment_id}/replies — Create a reply for a review comment ->201
  b: body*
