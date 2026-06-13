# deployments-environments

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/environments — List environments [pg]

GET /repos/{owner}/{repo}/environments/{environment_name} — Get an environment

PUT /repos/{owner}/{repo}/environments/{environment_name} — Create or update an environment
  b: deployment_branch_policy{protected_branches*,custom_branch_policies*} prevent_self_review:b reviewers:[o] wait_timer:i

DELETE /repos/{owner}/{repo}/environments/{environment_name} — Delete an environment ->204
