# repos-rules

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/rules/branches/{branch} — Get rules for a branch [pg]

GET /repos/{owner}/{repo}/rulesets — Get all repository rulesets [pg]
  q: includes_parents:b=true targets

POST /repos/{owner}/{repo}/rulesets — Create a repository ruleset ->201
  b: enforcement*(disabled|active|evaluate) name* bypass_actors:[o] conditions{} rules:[o] target(branch|tag|push)

GET /repos/{owner}/{repo}/rulesets/{ruleset_id} — Get a repository ruleset
  q: includes_parents:b=true

PUT /repos/{owner}/{repo}/rulesets/{ruleset_id} — Update a repository ruleset
  b: bypass_actors:[o] conditions{} enforcement(disabled|active|evaluate) name rules:[o] target(branch|tag|push)

DELETE /repos/{owner}/{repo}/rulesets/{ruleset_id} — Delete a repository ruleset ->204

GET /repos/{owner}/{repo}/rulesets/{ruleset_id}/history — Get repository ruleset history [pg]

GET /repos/{owner}/{repo}/rulesets/{ruleset_id}/history/{version_id} — Get repository ruleset version
