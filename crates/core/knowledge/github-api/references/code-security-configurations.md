# code-security-configurations

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /enterprises/{enterprise}/code-security/configurations — Get code security configurations for an enterprise [pg]
  q: before after

POST /enterprises/{enterprise}/code-security/configurations — Create a code security configuration for an enterprise ->201
  b: name* advanced_security(enabled|disabled|code_security|secret_protection) code_scanning_default_setup(enabled|disabled|not_set) code_scanning_default_setup_options{} code_scanning_delegated_alert_dismissal(enabled|disabled|not_set) code_scanning_options{} code_security(enabled|disabled|not_set) dependabot_alerts(enabled|disabled|not_set) dependabot_security_updates(enabled|disabled|not_set) dependency_graph(enabled|disabled|not_set) dependency_graph_autosubmit_action(enabled|disabled|not_set) dependency_graph_autosubmit_action_options{} description enforcement(enforced|unenforced) private_vulnerability_reporting(enabled|disabled|not_set) secret_protection(enabled|disabled|not_set) secret_scanning(enabled|disabled|not_set) secret_scanning_delegated_alert_dismissal(enabled|disabled|not_set) secret_scanning_extended_metadata(enabled|disabled|not_set) secret_scanning_generic_secrets(enabled|disabled|not_set) secret_scanning_non_provider_patterns(enabled|disabled|not_set) secret_scanning_push_protection(enabled|disabled|not_set) secret_scanning_validity_checks(enabled|disabled|not_set)

GET /enterprises/{enterprise}/code-security/configurations/defaults — Get default code security configurations for an enterprise

GET /enterprises/{enterprise}/code-security/configurations/{configuration_id} — Retrieve a code security configuration of an enterprise

PATCH /enterprises/{enterprise}/code-security/configurations/{configuration_id} — Update a custom code security configuration for an enterprise
  b: advanced_security(enabled|disabled|code_security|secret_protection) code_scanning_default_setup(enabled|disabled|not_set) code_scanning_default_setup_options{} code_scanning_delegated_alert_dismissal(enabled|disabled|not_set) code_scanning_options{} code_security(enabled|disabled|not_set) dependabot_alerts(enabled|disabled|not_set) dependabot_security_updates(enabled|disabled|not_set) dependency_graph(enabled|disabled|not_set) dependency_graph_autosubmit_action(enabled|disabled|not_set) dependency_graph_autosubmit_action_options{} description enforcement(enforced|unenforced) name private_vulnerability_reporting(enabled|disabled|not_set) secret_protection(enabled|disabled|not_set) secret_scanning(enabled|disabled|not_set) secret_scanning_delegated_alert_dismissal(enabled|disabled|not_set) secret_scanning_extended_metadata(enabled|disabled|not_set) secret_scanning_generic_secrets(enabled|disabled|not_set) secret_scanning_non_provider_patterns(enabled|disabled|not_set) secret_scanning_push_protection(enabled|disabled|not_set) secret_scanning_validity_checks(enabled|disabled|not_set)

DELETE /enterprises/{enterprise}/code-security/configurations/{configuration_id} — Delete a code security configuration for an enterprise ->204

POST /enterprises/{enterprise}/code-security/configurations/{configuration_id}/attach — Attach an enterprise configuration to repositories ->202
  b: scope*(all|all_without_configurations)

PUT /enterprises/{enterprise}/code-security/configurations/{configuration_id}/defaults — Set a code security configuration as a default for an enterprise
  b: default_for_new_repos(all|none|private_and_internal|public)

GET /enterprises/{enterprise}/code-security/configurations/{configuration_id}/repositories — Get repositories associated with an enterprise code security configuration [pg]
  q: before after status=all

GET /orgs/{org}/code-security/configurations — Get code security configurations for an organization [pg]
  q: target_type(global|all)=all before after

POST /orgs/{org}/code-security/configurations — Create a code security configuration ->201
  b: name* advanced_security(enabled|disabled|code_security|secret_protection) code_scanning_default_setup(enabled|disabled|not_set) code_scanning_default_setup_options{} code_scanning_delegated_alert_dismissal(enabled|disabled|not_set) code_scanning_options{} code_security(enabled|disabled|not_set) dependabot_alerts(enabled|disabled|not_set) dependabot_delegated_alert_dismissal(enabled|disabled|not_set) dependabot_security_updates(enabled|disabled|not_set) dependency_graph(enabled|disabled|not_set) dependency_graph_autosubmit_action(enabled|disabled|not_set) dependency_graph_autosubmit_action_options{} description enforcement(enforced|unenforced) private_vulnerability_reporting(enabled|disabled|not_set) secret_protection(enabled|disabled|not_set) secret_scanning(enabled|disabled|not_set) secret_scanning_delegated_alert_dismissal(enabled|disabled|not_set) secret_scanning_delegated_bypass(enabled|disabled|not_set) secret_scanning_delegated_bypass_options{} secret_scanning_extended_metadata(enabled|disabled|not_set) secret_scanning_generic_secrets(enabled|disabled|not_set) secret_scanning_non_provider_patterns(enabled|disabled|not_set) secret_scanning_push_protection(enabled|disabled|not_set) secret_scanning_validity_checks(enabled|disabled|not_set)

GET /orgs/{org}/code-security/configurations/defaults — Get default code security configurations

DELETE /orgs/{org}/code-security/configurations/detach — Detach configurations from repositories ->204
  b: selected_repository_ids*:[i]

GET /orgs/{org}/code-security/configurations/{configuration_id} — Get a code security configuration

PATCH /orgs/{org}/code-security/configurations/{configuration_id} — Update a code security configuration
  b: advanced_security(enabled|disabled|code_security|secret_protection) code_scanning_default_setup(enabled|disabled|not_set) code_scanning_default_setup_options{} code_scanning_delegated_alert_dismissal(enabled|disabled|not_set) code_scanning_options{} code_security(enabled|disabled|not_set) dependabot_alerts(enabled|disabled|not_set) dependabot_delegated_alert_dismissal(enabled|disabled|not_set) dependabot_security_updates(enabled|disabled|not_set) dependency_graph(enabled|disabled|not_set) dependency_graph_autosubmit_action(enabled|disabled|not_set) dependency_graph_autosubmit_action_options{} description enforcement(enforced|unenforced) name private_vulnerability_reporting(enabled|disabled|not_set) secret_protection(enabled|disabled|not_set) secret_scanning(enabled|disabled|not_set) secret_scanning_delegated_alert_dismissal(enabled|disabled|not_set) secret_scanning_delegated_bypass(enabled|disabled|not_set) secret_scanning_delegated_bypass_options{} secret_scanning_extended_metadata(enabled|disabled|not_set) secret_scanning_generic_secrets(enabled|disabled|not_set) secret_scanning_non_provider_patterns(enabled|disabled|not_set) secret_scanning_push_protection(enabled|disabled|not_set) secret_scanning_validity_checks(enabled|disabled|not_set)

DELETE /orgs/{org}/code-security/configurations/{configuration_id} — Delete a code security configuration ->204

POST /orgs/{org}/code-security/configurations/{configuration_id}/attach — Attach a configuration to repositories ->202
  b: scope*(all|all_without_configurations|public|private_or_internal|selected) selected_repository_ids:[i]

PUT /orgs/{org}/code-security/configurations/{configuration_id}/defaults — Set a code security configuration as a default for an organization
  b: default_for_new_repos(all|none|private_and_internal|public)

GET /orgs/{org}/code-security/configurations/{configuration_id}/repositories — Get repositories associated with a code security configuration [pg]
  q: before after status=all

GET /repos/{owner}/{repo}/code-security-configuration — Get the code security configuration associated with a repository
