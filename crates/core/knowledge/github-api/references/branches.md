# branches

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/branches — List branches [pg]
  q: protected:b

GET /repos/{owner}/{repo}/branches/{branch} — Get a branch

POST /repos/{owner}/{repo}/branches/{branch}/rename — Rename a branch ->201
  b: new_name*

POST /repos/{owner}/{repo}/merge-upstream — Sync a fork branch with the upstream repository
  b: branch*

POST /repos/{owner}/{repo}/merges — Merge a branch ->201
  b: base* head* commit_message
