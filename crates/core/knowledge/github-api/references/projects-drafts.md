# projects-drafts

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

POST /orgs/{org}/projectsV2/{project_number}/drafts — Create draft item for organization owned project ->201
  b: title* body

POST /user/{user_id}/projectsV2/{project_number}/drafts — Create draft item for user owned project ->201
  b: title* body
