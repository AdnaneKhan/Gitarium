# billing-budgets

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /organizations/{org}/settings/billing/budgets — Get all budgets for an organization [pg]
  q: scope(enterprise|organization|repository|cost_center|multi_user_customer|user) user

POST /organizations/{org}/settings/billing/budgets — Create a budget for an organization
  b: budget_alerting{} budget_amount:i budget_entity_name budget_product_sku budget_scope(organization|repository|multi_user_customer|user) budget_type prevent_further_usage:b

GET /organizations/{org}/settings/billing/budgets/{budget_id} — Get a budget by ID for an organization

PATCH /organizations/{org}/settings/billing/budgets/{budget_id} — Update a budget for an organization
  b: budget_alerting{} budget_amount:i budget_entity_name budget_product_sku budget_scope(enterprise|organization|repository|cost_center|multi_user_customer|user) budget_type prevent_further_usage:b

DELETE /organizations/{org}/settings/billing/budgets/{budget_id} — Delete a budget for an organization
