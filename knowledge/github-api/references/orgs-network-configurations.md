# orgs-network-configurations

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/settings/network-configurations — List hosted compute network configurations for an organization [pg]

POST /orgs/{org}/settings/network-configurations — Create a hosted compute network configuration for an organization ->201
  b: name* network_settings_ids*:[] compute_service(none|actions) failover_network_enabled:b failover_network_settings_ids:[]

GET /orgs/{org}/settings/network-configurations/{network_configuration_id} — Get a hosted compute network configuration for an organization

PATCH /orgs/{org}/settings/network-configurations/{network_configuration_id} — Update a hosted compute network configuration for an organization
  b: compute_service(none|actions) failover_network_enabled:b failover_network_settings_ids:[] name network_settings_ids:[]

DELETE /orgs/{org}/settings/network-configurations/{network_configuration_id} — Delete a hosted compute network configuration from an organization ->204

GET /orgs/{org}/settings/network-settings/{network_settings_id} — Get a hosted compute network settings resource for an organization
