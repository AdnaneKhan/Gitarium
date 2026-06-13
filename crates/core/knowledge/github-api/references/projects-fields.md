# projects-fields

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/projectsV2/{project_number}/fields — List project fields for organization [pg]
  q: before after

POST /orgs/{org}/projectsV2/{project_number}/fields — Add a field to an organization-owned project. ->201
  b: data_type(iteration) issue_field_id:i iteration_configuration{} name single_select_options:[o] (one-of)

GET /orgs/{org}/projectsV2/{project_number}/fields/{field_id} — Get project field for organization

GET /users/{username}/projectsV2/{project_number}/fields — List project fields for user [pg]
  q: before after

POST /users/{username}/projectsV2/{project_number}/fields — Add field to user owned project ->201
  b: data_type(iteration) iteration_configuration{} name single_select_options:[o] (one-of)

GET /users/{username}/projectsV2/{project_number}/fields/{field_id} — Get project field for user
