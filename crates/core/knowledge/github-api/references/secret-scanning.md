# secret-scanning

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/secret-scanning/alerts — List secret scanning alerts for an organization [pg]
  q: state(open|resolved) secret_type exclude_secret_types exclude_providers providers resolution assignee sort(created|updated)=created direction(asc|desc)=desc before after validity is_publicly_leaked:b=false is_multi_repo:b=false hide_secret:b=false is_bypassed:b

GET /repos/{owner}/{repo}/secret-scanning/alerts — List secret scanning alerts for a repository [pg]
  q: state(open|resolved) secret_type exclude_secret_types exclude_providers providers resolution assignee sort(created|updated)=created direction(asc|desc)=desc before after validity is_publicly_leaked:b=false is_multi_repo:b=false hide_secret:b=false is_bypassed:b

GET /repos/{owner}/{repo}/secret-scanning/alerts/{alert_number} — Get a secret scanning alert
  q: hide_secret:b=false

PATCH /repos/{owner}/{repo}/secret-scanning/alerts/{alert_number} — Update a secret scanning alert
  b: assignee resolution(false_positive|wont_fix|revoked|used_in_tests) resolution_comment state(open|resolved) validity(active|inactive)

GET /repos/{owner}/{repo}/secret-scanning/alerts/{alert_number}/locations — List locations for a secret scanning alert [pg]

POST /repos/{owner}/{repo}/secret-scanning/push-protection-bypasses — Create a push protection bypass
  b: placeholder_id* reason*(false_positive|used_in_tests|will_fix_later)

GET /repos/{owner}/{repo}/secret-scanning/scan-history — Get secret scanning scan history for a repository
