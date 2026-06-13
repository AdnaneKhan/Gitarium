# checks-suites

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

POST /repos/{owner}/{repo}/check-suites — Create a check suite
  b: head_sha*

PATCH /repos/{owner}/{repo}/check-suites/preferences — Update repository preferences for check suites
  b: auto_trigger_checks:[o]

GET /repos/{owner}/{repo}/check-suites/{check_suite_id} — Get a check suite

POST /repos/{owner}/{repo}/check-suites/{check_suite_id}/rerequest — Rerequest a check suite ->201

GET /repos/{owner}/{repo}/commits/{ref}/check-suites — List check suites for a Git reference [pg]
  q: app_id:i check_name
