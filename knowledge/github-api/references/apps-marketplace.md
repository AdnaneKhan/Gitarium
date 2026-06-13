# apps-marketplace

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /marketplace_listing/accounts/{account_id} — Get a subscription plan for an account

GET /marketplace_listing/plans — List plans [pg]

GET /marketplace_listing/plans/{plan_id}/accounts — List accounts for a plan [pg]
  q: sort(created|updated)=created direction(asc|desc)

GET /marketplace_listing/stubbed/accounts/{account_id} — Get a subscription plan for an account (stubbed)

GET /marketplace_listing/stubbed/plans — List plans (stubbed) [pg]

GET /marketplace_listing/stubbed/plans/{plan_id}/accounts — List accounts for a plan (stubbed) [pg]
  q: sort(created|updated)=created direction(asc|desc)

GET /user/marketplace_purchases — List subscriptions for the authenticated user [pg]

GET /user/marketplace_purchases/stubbed — List subscriptions for the authenticated user (stubbed) [pg]
