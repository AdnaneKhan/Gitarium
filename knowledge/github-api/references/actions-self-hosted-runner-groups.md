# actions-self-hosted-runner-groups

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/actions/runner-groups — List self-hosted runner groups for an organization [pg]
  q: visible_to_repository

POST /orgs/{org}/actions/runner-groups — Create a self-hosted runner group for an organization ->201
  b: name* allows_public_repositories:b network_configuration_id restricted_to_workflows:b runners:[i] selected_repository_ids:[i] selected_workflows:[] visibility(selected|all|private)

GET /orgs/{org}/actions/runner-groups/{runner_group_id} — Get a self-hosted runner group for an organization

PATCH /orgs/{org}/actions/runner-groups/{runner_group_id} — Update a self-hosted runner group for an organization
  b: name* allows_public_repositories:b network_configuration_id restricted_to_workflows:b selected_workflows:[] visibility(selected|all|private)

DELETE /orgs/{org}/actions/runner-groups/{runner_group_id} — Delete a self-hosted runner group from an organization ->204

GET /orgs/{org}/actions/runner-groups/{runner_group_id}/hosted-runners — List GitHub-hosted runners in a group for an organization [pg]

GET /orgs/{org}/actions/runner-groups/{runner_group_id}/repositories — List repository access to a self-hosted runner group in an organization [pg]

PUT /orgs/{org}/actions/runner-groups/{runner_group_id}/repositories — Set repository access for a self-hosted runner group in an organization ->204
  b: selected_repository_ids*:[i]

PUT /orgs/{org}/actions/runner-groups/{runner_group_id}/repositories/{repository_id} — Add repository access to a self-hosted runner group in an organization ->204

DELETE /orgs/{org}/actions/runner-groups/{runner_group_id}/repositories/{repository_id} — Remove repository access to a self-hosted runner group in an organization ->204

GET /orgs/{org}/actions/runner-groups/{runner_group_id}/runners — List self-hosted runners in a group for an organization [pg]

PUT /orgs/{org}/actions/runner-groups/{runner_group_id}/runners — Set self-hosted runners in a group for an organization ->204
  b: runners*:[i]

PUT /orgs/{org}/actions/runner-groups/{runner_group_id}/runners/{runner_id} — Add a self-hosted runner to a group for an organization ->204

DELETE /orgs/{org}/actions/runner-groups/{runner_group_id}/runners/{runner_id} — Remove a self-hosted runner from a group for an organization ->204
