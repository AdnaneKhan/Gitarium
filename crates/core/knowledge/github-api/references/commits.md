# commits

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/commits — List commits [pg]
  q: sha path author committer since until

GET /repos/{owner}/{repo}/commits/{commit_sha}/branches-where-head — List branches for HEAD commit

GET /repos/{owner}/{repo}/commits/{commit_sha}/pulls — List pull requests associated with a commit [pg]

GET /repos/{owner}/{repo}/commits/{ref} — Get a commit [pg]

GET /repos/{owner}/{repo}/compare/{basehead} — Compare two commits [pg]
