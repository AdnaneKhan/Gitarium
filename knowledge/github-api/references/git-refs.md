# git-refs

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/git/matching-refs/{ref} — List matching references

GET /repos/{owner}/{repo}/git/ref/{ref} — Get a reference

POST /repos/{owner}/{repo}/git/refs — Create a reference ->201
  b: ref* sha*

PATCH /repos/{owner}/{repo}/git/refs/{ref} — Update a reference
  b: sha* force:b

DELETE /repos/{owner}/{repo}/git/refs/{ref} — Delete a reference ->204
