# git-blobs

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

POST /repos/{owner}/{repo}/git/blobs — Create a blob ->201
  b: content* encoding

GET /repos/{owner}/{repo}/git/blobs/{file_sha} — Get a blob
