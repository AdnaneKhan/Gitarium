# gists

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /gists — List gists for the authenticated user [pg]
  q: since

POST /gists — Create a gist ->201
  b: files*{} description public

GET /gists/public — List public gists [pg]
  q: since

GET /gists/starred — List starred gists [pg]
  q: since

GET /gists/{gist_id} — Get a gist

PATCH /gists/{gist_id} — Update a gist
  b: description files{}

DELETE /gists/{gist_id} — Delete a gist ->204

GET /gists/{gist_id}/commits — List gist commits [pg]

GET /gists/{gist_id}/forks — List gist forks [pg]

POST /gists/{gist_id}/forks — Fork a gist ->201

GET /gists/{gist_id}/star — Check if a gist is starred ->204

PUT /gists/{gist_id}/star — Star a gist ->204

DELETE /gists/{gist_id}/star — Unstar a gist ->204

GET /gists/{gist_id}/{sha} — Get a gist revision

GET /users/{username}/gists — List gists for a user [pg]
  q: since
