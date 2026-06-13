# deployments-protection-rules

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/environments/{environment_name}/deployment_protection_rules — Get all deployment protection rules for an environment

POST /repos/{owner}/{repo}/environments/{environment_name}/deployment_protection_rules — Create a custom deployment protection rule on an environment ->201
  b: integration_id:i

GET /repos/{owner}/{repo}/environments/{environment_name}/deployment_protection_rules/apps — List custom deployment rule integrations available for an environment [pg]

GET /repos/{owner}/{repo}/environments/{environment_name}/deployment_protection_rules/{protection_rule_id} — Get a custom deployment protection rule

DELETE /repos/{owner}/{repo}/environments/{environment_name}/deployment_protection_rules/{protection_rule_id} — Disable a custom protection rule for an environment ->204
