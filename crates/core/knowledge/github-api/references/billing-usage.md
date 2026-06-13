# billing-usage

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /organizations/{org}/settings/billing/ai_credit/usage — Get billing AI credit usage report for an organization
  q: year:i month:i day:i user model product

GET /organizations/{org}/settings/billing/premium_request/usage — Get billing premium request usage report for an organization
  q: year:i month:i day:i user model product

GET /organizations/{org}/settings/billing/usage — Get billing usage report for an organization
  q: year:i month:i day:i

GET /organizations/{org}/settings/billing/usage/summary — Get billing usage summary for an organization
  q: year:i month:i day:i repository product sku

GET /users/{username}/settings/billing/ai_credit/usage — Get billing AI credit usage report for a user
  q: year:i month:i day:i model product

GET /users/{username}/settings/billing/premium_request/usage — Get billing premium request usage report for a user
  q: year:i month:i day:i model product

GET /users/{username}/settings/billing/usage — Get billing usage report for a user
  q: year:i month:i day:i

GET /users/{username}/settings/billing/usage/summary — Get billing usage summary for a user
  q: year:i month:i day:i repository product sku
