# interactions-repos

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/interaction-limits — Get interaction restrictions for a repository

PUT /repos/{owner}/{repo}/interaction-limits — Set interaction restrictions for a repository
  b: limit*(existing_users|contributors_only|collaborators_only) expiry(one_day|three_days|one_week|one_month|six_months)

DELETE /repos/{owner}/{repo}/interaction-limits — Remove interaction restrictions for a repository ->204
