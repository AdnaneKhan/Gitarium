# private-registries-organization-configurations

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/private-registries — List private registries for an organization [pg]

POST /orgs/{org}/private-registries — Create a private registry for an organization ->201
  b: registry_type*(maven_repository|nuget_feed|goproxy_server|npm_registry|rubygems_server|…) url* visibility*(all|private|selected) account_id api_host audience auth_type(token|username_password|oidc_azure|oidc_aws|oidc_jfrog|…) aws_region client_id domain domain_owner encrypted_value identity_mapping_name jfrog_oidc_provider_name key_id namespace replaces_base:b role_name selected_repository_ids:[i] service_account service_slug tenant_id username workload_identity_provider

GET /orgs/{org}/private-registries/public-key — Get private registries public key for an organization

GET /orgs/{org}/private-registries/{secret_name} — Get a private registry for an organization

PATCH /orgs/{org}/private-registries/{secret_name} — Update a private registry for an organization ->204
  b: account_id api_host audience auth_type(token|username_password|oidc_azure|oidc_aws|oidc_jfrog|…) aws_region client_id domain domain_owner encrypted_value identity_mapping_name jfrog_oidc_provider_name key_id namespace registry_type(maven_repository|nuget_feed|goproxy_server|npm_registry|rubygems_server|…) replaces_base:b role_name selected_repository_ids:[i] service_account service_slug tenant_id url username visibility(all|private|selected) workload_identity_provider

DELETE /orgs/{org}/private-registries/{secret_name} — Delete a private registry for an organization ->204
