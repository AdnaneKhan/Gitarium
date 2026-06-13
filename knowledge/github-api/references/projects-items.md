# projects-items

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/projectsV2/{project_number}/items — List items for an organization owned project [pg]
  q: q fields before after

POST /orgs/{org}/projectsV2/{project_number}/items — Add item to organization owned project ->201
  b: type*(Issue|PullRequest) id:i number:i owner repo

GET /orgs/{org}/projectsV2/{project_number}/items/{item_id} — Get an item for an organization owned project
  q: fields

PATCH /orgs/{org}/projectsV2/{project_number}/items/{item_id} — Update project item for organization
  b: fields*:[o]

DELETE /orgs/{org}/projectsV2/{project_number}/items/{item_id} — Delete project item for organization ->204

GET /orgs/{org}/projectsV2/{project_number}/views/{view_number}/items — List items for an organization project view [pg]
  q: fields before after

GET /users/{username}/projectsV2/{project_number}/items — List items for a user owned project [pg]
  q: before after q fields

POST /users/{username}/projectsV2/{project_number}/items — Add item to user owned project ->201
  b: type*(Issue|PullRequest) id:i number:i owner repo

GET /users/{username}/projectsV2/{project_number}/items/{item_id} — Get an item for a user owned project
  q: fields

PATCH /users/{username}/projectsV2/{project_number}/items/{item_id} — Update project item for user
  b: fields*:[o]

DELETE /users/{username}/projectsV2/{project_number}/items/{item_id} — Delete project item for user ->204

GET /users/{username}/projectsV2/{project_number}/views/{view_number}/items — List items for a user project view [pg]
  q: fields before after
