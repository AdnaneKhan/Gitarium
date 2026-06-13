# copilot-spaces-collaborators

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/copilot-spaces/{space_number}/collaborators — List collaborators for an organization Copilot Space

POST /orgs/{org}/copilot-spaces/{space_number}/collaborators — Add a collaborator to an organization Copilot Space ->201
  b: actor_identifier* actor_type*(User|Team) role*(reader|writer|admin)

PUT /orgs/{org}/copilot-spaces/{space_number}/collaborators/{actor_type}/{actor_identifier} — Set a collaborator role for an organization Copilot Space
  b: role*(reader|writer|admin|no_access)

DELETE /orgs/{org}/copilot-spaces/{space_number}/collaborators/{actor_type}/{actor_identifier} — Remove a collaborator from an organization Copilot Space ->204

GET /users/{username}/copilot-spaces/{space_number}/collaborators — List collaborators for a Copilot Space for a user

POST /users/{username}/copilot-spaces/{space_number}/collaborators — Add a collaborator to a Copilot Space for a user ->201
  b: actor_identifier* actor_type*(User|Team) role*(reader|writer|admin)

PUT /users/{username}/copilot-spaces/{space_number}/collaborators/{actor_type}/{actor_identifier} — Set a collaborator role for a Copilot Space for a user
  b: role*(reader|writer|admin|no_access)

DELETE /users/{username}/copilot-spaces/{space_number}/collaborators/{actor_type}/{actor_identifier} — Remove a collaborator from a Copilot Space for a user ->204
