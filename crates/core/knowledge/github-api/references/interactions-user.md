# interactions-user

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /user/interaction-limits — Get interaction restrictions for your public repositories

PUT /user/interaction-limits — Set interaction restrictions for your public repositories
  b: limit*(existing_users|contributors_only|collaborators_only) expiry(one_day|three_days|one_week|one_month|six_months)

DELETE /user/interaction-limits — Remove interaction restrictions from your public repositories ->204
