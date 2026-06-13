# actions-oidc

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /enterprises/{enterprise}/actions/oidc/customization/properties/repo — List OIDC custom property inclusions for an enterprise

POST /enterprises/{enterprise}/actions/oidc/customization/properties/repo — Create an OIDC custom property inclusion for an enterprise ->201
  b: custom_property_name*

DELETE /enterprises/{enterprise}/actions/oidc/customization/properties/repo/{custom_property_name} — Delete an OIDC custom property inclusion for an enterprise ->204

GET /orgs/{org}/actions/oidc/customization/properties/repo — List OIDC custom property inclusions for an organization

POST /orgs/{org}/actions/oidc/customization/properties/repo — Create an OIDC custom property inclusion for an organization ->201
  b: custom_property_name*

DELETE /orgs/{org}/actions/oidc/customization/properties/repo/{custom_property_name} — Delete an OIDC custom property inclusion for an organization ->204

GET /orgs/{org}/actions/oidc/customization/sub — Get the customization template for an OIDC subject claim for an organization

PUT /orgs/{org}/actions/oidc/customization/sub — Set the customization template for an OIDC subject claim for an organization ->201
  b: include_claim_keys:[] use_immutable_subject:b

GET /repos/{owner}/{repo}/actions/oidc/customization/sub — Get the customization template for an OIDC subject claim for a repository

PUT /repos/{owner}/{repo}/actions/oidc/customization/sub — Set the customization template for an OIDC subject claim for a repository ->201
  b: use_default*:b include_claim_keys:[] use_immutable_subject:b
