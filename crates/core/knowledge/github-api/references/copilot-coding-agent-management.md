# copilot-coding-agent-management

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

PUT /enterprises/{enterprise}/copilot/policies/coding_agent — Set the coding agent policy for an enterprise ->204
  b: policy_state*(enabled_for_all_orgs|disabled_for_all_orgs|enabled_for_selected_orgs|configured_by_org_admins)

POST /enterprises/{enterprise}/copilot/policies/coding_agent/organizations — Add organizations to the enterprise coding agent policy ->204
  b: custom_properties:[o] organizations:[]

DELETE /enterprises/{enterprise}/copilot/policies/coding_agent/organizations — Remove organizations from the enterprise coding agent policy ->204
  b: custom_properties:[o] organizations:[]

GET /orgs/{org}/copilot/coding-agent/permissions — Get Copilot cloud agent permissions for an organization

PUT /orgs/{org}/copilot/coding-agent/permissions — Set Copilot cloud agent permissions for an organization ->204
  b: enabled_repositories*(all|selected|none)

GET /orgs/{org}/copilot/coding-agent/permissions/repositories — List repositories enabled for Copilot cloud agent in an organization [pg]

PUT /orgs/{org}/copilot/coding-agent/permissions/repositories — Set selected repositories for Copilot cloud agent in an organization ->204
  b: selected_repository_ids*:[i]

PUT /orgs/{org}/copilot/coding-agent/permissions/repositories/{repository_id} — Enable a repository for Copilot cloud agent in an organization ->204

DELETE /orgs/{org}/copilot/coding-agent/permissions/repositories/{repository_id} — Disable a repository for Copilot cloud agent in an organization ->204
