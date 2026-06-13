# orgs-custom-properties

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/properties/schema — Get all custom properties for an organization

PATCH /orgs/{org}/properties/schema — Create or update custom properties for an organization
  b: properties*:[o]

GET /orgs/{org}/properties/schema/{custom_property_name} — Get a custom property for an organization

PUT /orgs/{org}/properties/schema/{custom_property_name} — Create or update a custom property for an organization
  b: value_type*(string|single_select|multi_select|true_false|url) allowed_values:[] default_value description require_explicit_values:b required:b values_editable_by(org_actors|org_and_repo_actors)

DELETE /orgs/{org}/properties/schema/{custom_property_name} — Remove a custom property for an organization ->204

GET /orgs/{org}/properties/values — List custom property values for organization repositories [pg]
  q: repository_query

PATCH /orgs/{org}/properties/values — Create or update custom property values for organization repositories ->204
  b: properties*:[o] repository_names*:[]
