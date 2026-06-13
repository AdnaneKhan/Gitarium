# copilot-usage-metrics

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /enterprises/{enterprise}/copilot/metrics/reports/enterprise-1-day — Get Copilot enterprise usage metrics for a specific day
  q: day*

GET /enterprises/{enterprise}/copilot/metrics/reports/enterprise-28-day/latest — Get Copilot enterprise usage metrics

GET /enterprises/{enterprise}/copilot/metrics/reports/user-teams-1-day — Get Copilot enterprise user-teams report for a specific day
  q: day*

GET /enterprises/{enterprise}/copilot/metrics/reports/users-1-day — Get Copilot users usage metrics for a specific day
  q: day*

GET /enterprises/{enterprise}/copilot/metrics/reports/users-28-day/latest — Get Copilot users usage metrics

GET /orgs/{org}/copilot/metrics/reports/organization-1-day — Get Copilot organization usage metrics for a specific day
  q: day*

GET /orgs/{org}/copilot/metrics/reports/organization-28-day/latest — Get Copilot organization usage metrics

GET /orgs/{org}/copilot/metrics/reports/user-teams-1-day — Get Copilot organization user-teams report for a specific day
  q: day*

GET /orgs/{org}/copilot/metrics/reports/users-1-day — Get Copilot organization users usage metrics for a specific day
  q: day*

GET /orgs/{org}/copilot/metrics/reports/users-28-day/latest — Get Copilot organization users usage metrics
