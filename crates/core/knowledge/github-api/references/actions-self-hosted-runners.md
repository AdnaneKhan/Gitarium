# actions-self-hosted-runners

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/actions/runners — List self-hosted runners for an organization [pg]
  q: name

GET /orgs/{org}/actions/runners/downloads — List runner applications for an organization

POST /orgs/{org}/actions/runners/generate-jitconfig — Create configuration for a just-in-time runner for an organization ->201
  b: labels*:[] name* runner_group_id*:i work_folder

POST /orgs/{org}/actions/runners/registration-token — Create a registration token for an organization ->201

POST /orgs/{org}/actions/runners/remove-token — Create a remove token for an organization ->201

GET /orgs/{org}/actions/runners/{runner_id} — Get a self-hosted runner for an organization

DELETE /orgs/{org}/actions/runners/{runner_id} — Delete a self-hosted runner from an organization ->204

GET /orgs/{org}/actions/runners/{runner_id}/labels — List labels for a self-hosted runner for an organization

PUT /orgs/{org}/actions/runners/{runner_id}/labels — Set custom labels for a self-hosted runner for an organization
  b: labels*:[]

POST /orgs/{org}/actions/runners/{runner_id}/labels — Add custom labels to a self-hosted runner for an organization
  b: labels*:[]

DELETE /orgs/{org}/actions/runners/{runner_id}/labels — Remove all custom labels from a self-hosted runner for an organization

DELETE /orgs/{org}/actions/runners/{runner_id}/labels/{name} — Remove a custom label from a self-hosted runner for an organization

GET /repos/{owner}/{repo}/actions/runners — List self-hosted runners for a repository [pg]
  q: name

GET /repos/{owner}/{repo}/actions/runners/downloads — List runner applications for a repository

POST /repos/{owner}/{repo}/actions/runners/generate-jitconfig — Create configuration for a just-in-time runner for a repository ->201
  b: labels*:[] name* runner_group_id*:i work_folder

POST /repos/{owner}/{repo}/actions/runners/registration-token — Create a registration token for a repository ->201

POST /repos/{owner}/{repo}/actions/runners/remove-token — Create a remove token for a repository ->201

GET /repos/{owner}/{repo}/actions/runners/{runner_id} — Get a self-hosted runner for a repository

DELETE /repos/{owner}/{repo}/actions/runners/{runner_id} — Delete a self-hosted runner from a repository ->204

GET /repos/{owner}/{repo}/actions/runners/{runner_id}/labels — List labels for a self-hosted runner for a repository

PUT /repos/{owner}/{repo}/actions/runners/{runner_id}/labels — Set custom labels for a self-hosted runner for a repository
  b: labels*:[]

POST /repos/{owner}/{repo}/actions/runners/{runner_id}/labels — Add custom labels to a self-hosted runner for a repository
  b: labels*:[]

DELETE /repos/{owner}/{repo}/actions/runners/{runner_id}/labels — Remove all custom labels from a self-hosted runner for a repository

DELETE /repos/{owner}/{repo}/actions/runners/{runner_id}/labels/{name} — Remove a custom label from a self-hosted runner for a repository
