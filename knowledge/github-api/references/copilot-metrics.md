# copilot-metrics

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/copilot/metrics — Get Copilot metrics for an organization [pg]
  q: since until

GET /orgs/{org}/team/{team_slug}/copilot/metrics — Get Copilot metrics for a team [pg]
  q: since until
