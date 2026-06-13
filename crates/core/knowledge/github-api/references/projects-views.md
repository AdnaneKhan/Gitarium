# projects-views

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

POST /orgs/{org}/projectsV2/{project_number}/views — Create a view for an organization-owned project ->201
  b: layout*(table|board|roadmap) name* filter visible_fields:[i]

POST /users/{user_id}/projectsV2/{project_number}/views — Create a view for a user-owned project ->201
  b: layout*(table|board|roadmap) name* filter visible_fields:[i]
