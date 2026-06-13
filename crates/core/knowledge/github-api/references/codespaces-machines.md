# codespaces-machines

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/codespaces/machines — List available machine types for a repository
  q: location client_ip ref

GET /user/codespaces/{codespace_name}/machines — List machine types for a codespace
