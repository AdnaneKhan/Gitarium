# issues-sub-issues

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/issues/{issue_number}/parent — Get parent issue

DELETE /repos/{owner}/{repo}/issues/{issue_number}/sub_issue — Remove sub-issue
  b: sub_issue_id*:i

GET /repos/{owner}/{repo}/issues/{issue_number}/sub_issues — List sub-issues [pg]

POST /repos/{owner}/{repo}/issues/{issue_number}/sub_issues — Add sub-issue ->201
  b: sub_issue_id*:i replace_parent:b

PATCH /repos/{owner}/{repo}/issues/{issue_number}/sub_issues/priority — Reprioritize sub-issue
  b: sub_issue_id*:i after_id:i before_id:i
