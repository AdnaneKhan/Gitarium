# actions-variables

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/actions/variables — List organization variables [pg]

POST /orgs/{org}/actions/variables — Create an organization variable ->201
  b: name* value* visibility*(all|private|selected) selected_repository_ids:[i]

GET /orgs/{org}/actions/variables/{name} — Get an organization variable

PATCH /orgs/{org}/actions/variables/{name} — Update an organization variable ->204
  b: name selected_repository_ids:[i] value visibility(all|private|selected)

DELETE /orgs/{org}/actions/variables/{name} — Delete an organization variable ->204

GET /orgs/{org}/actions/variables/{name}/repositories — List selected repositories for an organization variable [pg]

PUT /orgs/{org}/actions/variables/{name}/repositories — Set selected repositories for an organization variable ->204
  b: selected_repository_ids*:[i]

PUT /orgs/{org}/actions/variables/{name}/repositories/{repository_id} — Add selected repository to an organization variable ->204

DELETE /orgs/{org}/actions/variables/{name}/repositories/{repository_id} — Remove selected repository from an organization variable ->204

GET /repos/{owner}/{repo}/actions/organization-variables — List repository organization variables [pg]

GET /repos/{owner}/{repo}/actions/variables — List repository variables [pg]

POST /repos/{owner}/{repo}/actions/variables — Create a repository variable ->201
  b: name* value*

GET /repos/{owner}/{repo}/actions/variables/{name} — Get a repository variable

PATCH /repos/{owner}/{repo}/actions/variables/{name} — Update a repository variable ->204
  b: name value

DELETE /repos/{owner}/{repo}/actions/variables/{name} — Delete a repository variable ->204

GET /repos/{owner}/{repo}/environments/{environment_name}/variables — List environment variables [pg]

POST /repos/{owner}/{repo}/environments/{environment_name}/variables — Create an environment variable ->201
  b: name* value*

GET /repos/{owner}/{repo}/environments/{environment_name}/variables/{name} — Get an environment variable

PATCH /repos/{owner}/{repo}/environments/{environment_name}/variables/{name} — Update an environment variable ->204
  b: name value

DELETE /repos/{owner}/{repo}/environments/{environment_name}/variables/{name} — Delete an environment variable ->204
