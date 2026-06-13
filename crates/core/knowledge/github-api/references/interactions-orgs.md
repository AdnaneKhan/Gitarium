# interactions-orgs

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/interaction-limits — Get interaction restrictions for an organization

PUT /orgs/{org}/interaction-limits — Set interaction restrictions for an organization
  b: limit*(existing_users|contributors_only|collaborators_only) expiry(one_day|three_days|one_week|one_month|six_months)

DELETE /orgs/{org}/interaction-limits — Remove interaction restrictions for an organization ->204
