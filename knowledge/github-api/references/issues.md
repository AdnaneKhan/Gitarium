# issues

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /issues — List issues assigned to the authenticated user [pg]
  q: filter(assigned|created|mentioned|subscribed|repos|all)=assigned state(open|closed|all)=open labels sort(created|updated|comments)=created direction(asc|desc)=desc since collab:b orgs:b owned:b pulls:b

GET /orgs/{org}/issues — List organization issues assigned to the authenticated user [pg]
  q: filter(assigned|created|mentioned|subscribed|repos|all)=assigned state(open|closed|all)=open labels type sort(created|updated|comments)=created direction(asc|desc)=desc since

GET /repos/{owner}/{repo}/issues — List repository issues [pg]
  q: milestone state(open|closed|all)=open assignee type creator mentioned issue_field_values labels sort(created|updated|comments)=created direction(asc|desc)=desc since

POST /repos/{owner}/{repo}/issues — Create an issue ->201
  b: title* assignee assignees:[] body issue_field_values:[o] labels:[] milestone type

GET /repos/{owner}/{repo}/issues/{issue_number} — Get an issue

PATCH /repos/{owner}/{repo}/issues/{issue_number} — Update an issue
  b: assignee assignees:[] body issue_field_values:[o] labels:[] milestone state(open|closed) state_reason(completed|not_planned|duplicate|reopened) title type

PUT /repos/{owner}/{repo}/issues/{issue_number}/lock — Lock an issue ->204
  b: lock_reason(off-topic|too heated|resolved|spam)

DELETE /repos/{owner}/{repo}/issues/{issue_number}/lock — Unlock an issue ->204

GET /user/issues — List user account issues assigned to the authenticated user [pg]
  q: filter(assigned|created|mentioned|subscribed|repos|all)=assigned state(open|closed|all)=open labels sort(created|updated|comments)=created direction(asc|desc)=desc since
