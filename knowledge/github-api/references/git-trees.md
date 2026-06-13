# git-trees

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

POST /repos/{owner}/{repo}/git/trees — Create a tree ->201
  b: tree*:[o] base_tree

GET /repos/{owner}/{repo}/git/trees/{tree_sha} — Get a tree
  q: recursive
