# agents-variables

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/agents/variables — List organization variables [pg]

POST /orgs/{org}/agents/variables — Create an organization variable ->201
  b: name* value* visibility*(all|private|selected) selected_repository_ids:[i]

GET /orgs/{org}/agents/variables/{name} — Get an organization variable

PATCH /orgs/{org}/agents/variables/{name} — Update an organization variable ->204
  b: name selected_repository_ids:[i] value visibility(all|private|selected)

DELETE /orgs/{org}/agents/variables/{name} — Delete an organization variable ->204

GET /orgs/{org}/agents/variables/{name}/repositories — List selected repositories for an organization variable [pg]

PUT /orgs/{org}/agents/variables/{name}/repositories — Set selected repositories for an organization variable ->204
  b: selected_repository_ids*:[i]

PUT /orgs/{org}/agents/variables/{name}/repositories/{repository_id} — Add selected repository to an organization variable ->204

DELETE /orgs/{org}/agents/variables/{name}/repositories/{repository_id} — Remove selected repository from an organization variable ->204

GET /repos/{owner}/{repo}/agents/organization-variables — List repository organization variables [pg]

GET /repos/{owner}/{repo}/agents/variables — List repository variables [pg]

POST /repos/{owner}/{repo}/agents/variables — Create a repository variable ->201
  b: name* value*

GET /repos/{owner}/{repo}/agents/variables/{name} — Get a repository variable

PATCH /repos/{owner}/{repo}/agents/variables/{name} — Update a repository variable ->204
  b: name value

DELETE /repos/{owner}/{repo}/agents/variables/{name} — Delete a repository variable ->204
