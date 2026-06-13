# orgs-rules

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/rulesets — Get all organization repository rulesets [pg]
  q: targets

POST /orgs/{org}/rulesets — Create an organization repository ruleset ->201
  b: enforcement*(disabled|active|evaluate) name* bypass_actors:[o] conditions{} rules:[o] target(branch|tag|push|repository)

GET /orgs/{org}/rulesets/{ruleset_id} — Get an organization repository ruleset

PUT /orgs/{org}/rulesets/{ruleset_id} — Update an organization repository ruleset
  b: bypass_actors:[o] conditions{} enforcement(disabled|active|evaluate) name rules:[o] target(branch|tag|push|repository)

DELETE /orgs/{org}/rulesets/{ruleset_id} — Delete an organization repository ruleset ->204

GET /orgs/{org}/rulesets/{ruleset_id}/history — Get organization ruleset history [pg]

GET /orgs/{org}/rulesets/{ruleset_id}/history/{version_id} — Get organization ruleset version
