# copilot-spaces

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/copilot-spaces — List organization Copilot Spaces [pg]
  q: before after

POST /orgs/{org}/copilot-spaces — Create an organization Copilot Space ->201
  b: name* base_role(reader|writer|admin|no_access) description general_instructions resources_attributes:[o]

GET /orgs/{org}/copilot-spaces/{space_number} — Get an organization Copilot Space

PUT /orgs/{org}/copilot-spaces/{space_number} — Set an organization Copilot Space
  b: base_role(reader|writer|admin|no_access) description general_instructions name resources_attributes:[o]

DELETE /orgs/{org}/copilot-spaces/{space_number} — Delete an organization Copilot Space ->204

GET /users/{username}/copilot-spaces — List Copilot Spaces for a user [pg]
  q: before after

POST /users/{username}/copilot-spaces — Create a Copilot Space for a user ->201
  b: name* base_role(reader|no_access) description general_instructions resources_attributes:[o]

GET /users/{username}/copilot-spaces/{space_number} — Get a Copilot Space for a user

PUT /users/{username}/copilot-spaces/{space_number} — Set a Copilot Space for a user
  b: base_role(reader|no_access) description general_instructions name resources_attributes:[o]

DELETE /users/{username}/copilot-spaces/{space_number} — Delete a Copilot Space for a user ->204
