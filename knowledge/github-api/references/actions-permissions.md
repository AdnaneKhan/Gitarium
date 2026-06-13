# actions-permissions

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/actions/permissions — Get GitHub Actions permissions for an organization

PUT /orgs/{org}/actions/permissions — Set GitHub Actions permissions for an organization ->204
  b: enabled_repositories*(all|none|selected) allowed_actions(all|local_only|selected) sha_pinning_required:b

GET /orgs/{org}/actions/permissions/artifact-and-log-retention — Get artifact and log retention settings for an organization

PUT /orgs/{org}/actions/permissions/artifact-and-log-retention — Set artifact and log retention settings for an organization ->204
  b: days*:i

GET /orgs/{org}/actions/permissions/fork-pr-contributor-approval — Get fork PR contributor approval permissions for an organization

PUT /orgs/{org}/actions/permissions/fork-pr-contributor-approval — Set fork PR contributor approval permissions for an organization ->204
  b: approval_policy*(first_time_contributors_new_to_github|first_time_contributors|all_external_contributors)

GET /orgs/{org}/actions/permissions/fork-pr-workflows-private-repos — Get private repo fork PR workflow settings for an organization

PUT /orgs/{org}/actions/permissions/fork-pr-workflows-private-repos — Set private repo fork PR workflow settings for an organization ->204
  b: run_workflows_from_fork_pull_requests*:b require_approval_for_fork_pr_workflows:b send_secrets_and_variables:b send_write_tokens_to_workflows:b

GET /orgs/{org}/actions/permissions/repositories — List selected repositories enabled for GitHub Actions in an organization [pg]

PUT /orgs/{org}/actions/permissions/repositories — Set selected repositories enabled for GitHub Actions in an organization ->204
  b: selected_repository_ids*:[i]

PUT /orgs/{org}/actions/permissions/repositories/{repository_id} — Enable a selected repository for GitHub Actions in an organization ->204

DELETE /orgs/{org}/actions/permissions/repositories/{repository_id} — Disable a selected repository for GitHub Actions in an organization ->204

GET /orgs/{org}/actions/permissions/selected-actions — Get allowed actions and reusable workflows for an organization

PUT /orgs/{org}/actions/permissions/selected-actions — Set allowed actions and reusable workflows for an organization ->204
  b: github_owned_allowed:b patterns_allowed:[] verified_allowed:b

GET /orgs/{org}/actions/permissions/self-hosted-runners — Get self-hosted runners settings for an organization

PUT /orgs/{org}/actions/permissions/self-hosted-runners — Set self-hosted runners settings for an organization ->204
  b: enabled_repositories*(all|selected|none)

GET /orgs/{org}/actions/permissions/self-hosted-runners/repositories — List repositories allowed to use self-hosted runners in an organization [pg]

PUT /orgs/{org}/actions/permissions/self-hosted-runners/repositories — Set repositories allowed to use self-hosted runners in an organization ->204
  b: selected_repository_ids*:[i]

PUT /orgs/{org}/actions/permissions/self-hosted-runners/repositories/{repository_id} — Add a repository to the list of repositories allowed to use self-hosted runners in an organization ->204

DELETE /orgs/{org}/actions/permissions/self-hosted-runners/repositories/{repository_id} — Remove a repository from the list of repositories allowed to use self-hosted runners in an organization ->204

GET /orgs/{org}/actions/permissions/workflow — Get default workflow permissions for an organization

PUT /orgs/{org}/actions/permissions/workflow — Set default workflow permissions for an organization ->204
  b: can_approve_pull_request_reviews:b default_workflow_permissions(read|write)

GET /repos/{owner}/{repo}/actions/permissions — Get GitHub Actions permissions for a repository

PUT /repos/{owner}/{repo}/actions/permissions — Set GitHub Actions permissions for a repository ->204
  b: enabled*:b allowed_actions(all|local_only|selected) sha_pinning_required:b

GET /repos/{owner}/{repo}/actions/permissions/access — Get the level of access for workflows outside of the repository

PUT /repos/{owner}/{repo}/actions/permissions/access — Set the level of access for workflows outside of the repository ->204
  b: access_level*(none|user|organization)

GET /repos/{owner}/{repo}/actions/permissions/artifact-and-log-retention — Get artifact and log retention settings for a repository

PUT /repos/{owner}/{repo}/actions/permissions/artifact-and-log-retention — Set artifact and log retention settings for a repository ->204
  b: days*:i

GET /repos/{owner}/{repo}/actions/permissions/fork-pr-contributor-approval — Get fork PR contributor approval permissions for a repository

PUT /repos/{owner}/{repo}/actions/permissions/fork-pr-contributor-approval — Set fork PR contributor approval permissions for a repository ->204
  b: approval_policy*(first_time_contributors_new_to_github|first_time_contributors|all_external_contributors)

GET /repos/{owner}/{repo}/actions/permissions/fork-pr-workflows-private-repos — Get private repo fork PR workflow settings for a repository

PUT /repos/{owner}/{repo}/actions/permissions/fork-pr-workflows-private-repos — Set private repo fork PR workflow settings for a repository ->204
  b: run_workflows_from_fork_pull_requests*:b require_approval_for_fork_pr_workflows:b send_secrets_and_variables:b send_write_tokens_to_workflows:b

GET /repos/{owner}/{repo}/actions/permissions/selected-actions — Get allowed actions and reusable workflows for a repository

PUT /repos/{owner}/{repo}/actions/permissions/selected-actions — Set allowed actions and reusable workflows for a repository ->204
  b: github_owned_allowed:b patterns_allowed:[] verified_allowed:b

GET /repos/{owner}/{repo}/actions/permissions/workflow — Get default workflow permissions for a repository

PUT /repos/{owner}/{repo}/actions/permissions/workflow — Set default workflow permissions for a repository ->204
  b: can_approve_pull_request_reviews:b default_workflow_permissions(read|write)
