# dependabot-alerts

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /enterprises/{enterprise}/dependabot/alerts — List Dependabot alerts for an enterprise [pg]
  q: classification state severity ecosystem package epss_percentage has assignee scope(development|runtime) sort(created|updated|epss_percentage)=created direction(asc|desc)=desc before after

GET /orgs/{org}/dependabot/alerts — List Dependabot alerts for an organization [pg]
  q: classification state severity ecosystem package epss_percentage artifact_registry_url artifact_registry has assignee runtime_risk scope(development|runtime) sort(created|updated|epss_percentage)=created direction(asc|desc)=desc before after

GET /repos/{owner}/{repo}/dependabot/alerts — List Dependabot alerts for a repository [pg]
  q: classification state severity ecosystem package manifest epss_percentage has assignee scope(development|runtime) sort(created|updated|epss_percentage)=created direction(asc|desc)=desc before after

GET /repos/{owner}/{repo}/dependabot/alerts/{alert_number} — Get a Dependabot alert

PATCH /repos/{owner}/{repo}/dependabot/alerts/{alert_number} — Update a Dependabot alert
  b: assignees:[] dismissed_comment dismissed_reason(fix_started|inaccurate|no_bandwidth|not_used|tolerable_risk) state(dismissed|open)
