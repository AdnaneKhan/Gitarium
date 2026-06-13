# deployments-branch-policies

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/environments/{environment_name}/deployment-branch-policies — List deployment branch policies [pg]

POST /repos/{owner}/{repo}/environments/{environment_name}/deployment-branch-policies — Create a deployment branch policy
  b: name* type(branch|tag)

GET /repos/{owner}/{repo}/environments/{environment_name}/deployment-branch-policies/{branch_policy_id} — Get a deployment branch policy

PUT /repos/{owner}/{repo}/environments/{environment_name}/deployment-branch-policies/{branch_policy_id} — Update a deployment branch policy
  b: name*

DELETE /repos/{owner}/{repo}/environments/{environment_name}/deployment-branch-policies/{branch_policy_id} — Delete a deployment branch policy ->204
