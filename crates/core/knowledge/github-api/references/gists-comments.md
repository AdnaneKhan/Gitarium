# gists-comments

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /gists/{gist_id}/comments — List gist comments [pg]

POST /gists/{gist_id}/comments — Create a gist comment ->201
  b: body*

GET /gists/{gist_id}/comments/{comment_id} — Get a gist comment

PATCH /gists/{gist_id}/comments/{comment_id} — Update a gist comment
  b: body*

DELETE /gists/{gist_id}/comments/{comment_id} — Delete a gist comment ->204
