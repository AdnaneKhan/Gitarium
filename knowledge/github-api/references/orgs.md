# orgs

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /organizations — List organizations [pg]
  q: since:i

GET /orgs/{org} — Get an organization

PATCH /orgs/{org} — Update an organization
  b: advanced_security_enabled_for_new_repositories:b billing_email blog company default_repository_permission(read|write|admin|none) dependabot_alerts_enabled_for_new_repositories:b dependabot_security_updates_enabled_for_new_repositories:b dependency_graph_enabled_for_new_repositories:b deploy_keys_enabled_for_repositories:b description email has_organization_projects:b has_repository_projects:b location members_allowed_repository_creation_type(all|private|none) members_can_create_internal_repositories:b members_can_create_pages:b members_can_create_private_pages:b members_can_create_private_repositories:b members_can_create_public_pages:b members_can_create_public_repositories:b members_can_create_repositories:b members_can_fork_private_repositories:b name secret_scanning_enabled_for_new_repositories:b secret_scanning_push_protection_custom_link secret_scanning_push_protection_custom_link_enabled:b secret_scanning_push_protection_enabled_for_new_repositories:b twitter_username web_commit_signoff_required:b

DELETE /orgs/{org} — Delete an organization ->202

GET /orgs/{org}/installations — List app installations for an organization [pg]

GET /orgs/{org}/settings/immutable-releases — Get immutable releases settings for an organization

PUT /orgs/{org}/settings/immutable-releases — Set immutable releases settings for an organization ->204
  b: enforced_repositories*(all|none|selected) selected_repository_ids:[i]

GET /orgs/{org}/settings/immutable-releases/repositories — List selected repositories for immutable releases enforcement [pg]

PUT /orgs/{org}/settings/immutable-releases/repositories — Set selected repositories for immutable releases enforcement ->204
  b: selected_repository_ids*:[i]

PUT /orgs/{org}/settings/immutable-releases/repositories/{repository_id} — Enable a selected repository for immutable releases in an organization ->204

DELETE /orgs/{org}/settings/immutable-releases/repositories/{repository_id} — Disable a selected repository for immutable releases in an organization ->204

POST /orgs/{org}/{security_product}/{enablement} — Enable or disable a security feature for an organization ->204 (deprecated)
  b: query_suite(default|extended)

GET /user/orgs — List organizations for the authenticated user [pg]

GET /users/{username}/orgs — List organizations for a user [pg]
