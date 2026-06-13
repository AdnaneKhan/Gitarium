# licenses

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /licenses — Get all commonly used licenses [pg]
  q: featured:b

GET /licenses/{license} — Get a license

GET /repos/{owner}/{repo}/license — Get the license for a repository
  q: ref
