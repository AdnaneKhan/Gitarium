# git-commits

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

POST /repos/{owner}/{repo}/git/commits — Create a commit ->201
  b: message* tree* author{name*,email*} committer{} parents:[] signature

GET /repos/{owner}/{repo}/git/commits/{commit_sha} — Get a commit object
