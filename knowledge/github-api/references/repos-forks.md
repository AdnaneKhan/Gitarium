# repos-forks

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/forks — List forks [pg]
  q: sort(newest|oldest|stargazers|watchers)=newest

POST /repos/{owner}/{repo}/forks — Create a fork ->202
  b: default_branch_only:b name organization
