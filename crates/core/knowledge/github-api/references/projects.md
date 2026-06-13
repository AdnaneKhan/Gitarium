# projects

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/projectsV2 — List projects for organization [pg]
  q: q before after

GET /orgs/{org}/projectsV2/{project_number} — Get project for organization

GET /users/{username}/projectsV2 — List projects for user [pg]
  q: q before after

GET /users/{username}/projectsV2/{project_number} — Get project for user
