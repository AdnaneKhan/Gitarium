# copilot-spaces-resources

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/copilot-spaces/{space_number}/resources — List resources for an organization Copilot Space

POST /orgs/{org}/copilot-spaces/{space_number}/resources — Create a resource for an organization Copilot Space
  b: metadata*{} resource_type*(repository|github_file|free_text|github_issue|github_pull_request)

GET /orgs/{org}/copilot-spaces/{space_number}/resources/{space_resource_id} — Get a resource for an organization Copilot Space

PUT /orgs/{org}/copilot-spaces/{space_number}/resources/{space_resource_id} — Set a resource for an organization Copilot Space
  b: metadata{}

DELETE /orgs/{org}/copilot-spaces/{space_number}/resources/{space_resource_id} — Delete a resource from an organization Copilot Space ->204

GET /users/{username}/copilot-spaces/{space_number}/resources — List resources for a Copilot Space for a user

POST /users/{username}/copilot-spaces/{space_number}/resources — Create a resource for a Copilot Space for a user
  b: metadata*{} resource_type*(repository|github_file|free_text|github_issue|github_pull_request)

GET /users/{username}/copilot-spaces/{space_number}/resources/{space_resource_id} — Get a resource for a Copilot Space for a user

PUT /users/{username}/copilot-spaces/{space_number}/resources/{space_resource_id} — Set a resource for a Copilot Space for a user
  b: metadata{}

DELETE /users/{username}/copilot-spaces/{space_number}/resources/{space_resource_id} — Delete a resource from a Copilot Space for a user ->204
