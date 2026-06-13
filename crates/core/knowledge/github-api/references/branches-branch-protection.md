# branches-branch-protection

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/branches/{branch}/protection — Get branch protection

PUT /repos/{owner}/{repo}/branches/{branch}/protection — Update branch protection
  b: enforce_admins*:b required_pull_request_reviews*{} required_status_checks*{strict*,contexts*} restrictions*{users*,teams*} allow_deletions:b allow_force_pushes:b allow_fork_syncing:b block_creations:b lock_branch:b required_conversation_resolution:b required_linear_history:b

DELETE /repos/{owner}/{repo}/branches/{branch}/protection — Delete branch protection ->204

GET /repos/{owner}/{repo}/branches/{branch}/protection/enforce_admins — Get admin branch protection

POST /repos/{owner}/{repo}/branches/{branch}/protection/enforce_admins — Set admin branch protection

DELETE /repos/{owner}/{repo}/branches/{branch}/protection/enforce_admins — Delete admin branch protection ->204

GET /repos/{owner}/{repo}/branches/{branch}/protection/required_pull_request_reviews — Get pull request review protection

PATCH /repos/{owner}/{repo}/branches/{branch}/protection/required_pull_request_reviews — Update pull request review protection
  b: bypass_pull_request_allowances{} dismiss_stale_reviews:b dismissal_restrictions{} require_code_owner_reviews:b require_last_push_approval:b required_approving_review_count:i

DELETE /repos/{owner}/{repo}/branches/{branch}/protection/required_pull_request_reviews — Delete pull request review protection ->204

GET /repos/{owner}/{repo}/branches/{branch}/protection/required_signatures — Get commit signature protection

POST /repos/{owner}/{repo}/branches/{branch}/protection/required_signatures — Create commit signature protection

DELETE /repos/{owner}/{repo}/branches/{branch}/protection/required_signatures — Delete commit signature protection ->204

GET /repos/{owner}/{repo}/branches/{branch}/protection/required_status_checks — Get status checks protection

PATCH /repos/{owner}/{repo}/branches/{branch}/protection/required_status_checks — Update status check protection
  b: checks:[o] contexts:[] strict:b

DELETE /repos/{owner}/{repo}/branches/{branch}/protection/required_status_checks — Remove status check protection ->204

GET /repos/{owner}/{repo}/branches/{branch}/protection/required_status_checks/contexts — Get all status check contexts

PUT /repos/{owner}/{repo}/branches/{branch}/protection/required_status_checks/contexts — Set status check contexts
  b: contexts:[] (one-of)

POST /repos/{owner}/{repo}/branches/{branch}/protection/required_status_checks/contexts — Add status check contexts
  b: contexts:[] (one-of)

DELETE /repos/{owner}/{repo}/branches/{branch}/protection/required_status_checks/contexts — Remove status check contexts
  b: contexts:[] (one-of)

GET /repos/{owner}/{repo}/branches/{branch}/protection/restrictions — Get access restrictions

DELETE /repos/{owner}/{repo}/branches/{branch}/protection/restrictions — Delete access restrictions ->204

GET /repos/{owner}/{repo}/branches/{branch}/protection/restrictions/apps — Get apps with access to the protected branch

PUT /repos/{owner}/{repo}/branches/{branch}/protection/restrictions/apps — Set app access restrictions
  b: apps*:[]

POST /repos/{owner}/{repo}/branches/{branch}/protection/restrictions/apps — Add app access restrictions
  b: apps*:[]

DELETE /repos/{owner}/{repo}/branches/{branch}/protection/restrictions/apps — Remove app access restrictions
  b: apps*:[]

GET /repos/{owner}/{repo}/branches/{branch}/protection/restrictions/teams — Get teams with access to the protected branch

PUT /repos/{owner}/{repo}/branches/{branch}/protection/restrictions/teams — Set team access restrictions
  b: teams:[] (one-of)

POST /repos/{owner}/{repo}/branches/{branch}/protection/restrictions/teams — Add team access restrictions
  b: teams:[] (one-of)

DELETE /repos/{owner}/{repo}/branches/{branch}/protection/restrictions/teams — Remove team access restrictions
  b: teams:[] (one-of)

GET /repos/{owner}/{repo}/branches/{branch}/protection/restrictions/users — Get users with access to the protected branch

PUT /repos/{owner}/{repo}/branches/{branch}/protection/restrictions/users — Set user access restrictions
  b: users*:[]

POST /repos/{owner}/{repo}/branches/{branch}/protection/restrictions/users — Add user access restrictions
  b: users*:[]

DELETE /repos/{owner}/{repo}/branches/{branch}/protection/restrictions/users — Remove user access restrictions
  b: users*:[]
