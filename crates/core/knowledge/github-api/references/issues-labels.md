# issues-labels

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/issues/{issue_number}/labels — List labels for an issue [pg]

PUT /repos/{owner}/{repo}/issues/{issue_number}/labels — Set labels for an issue
  b: labels:[o] (one-of)

POST /repos/{owner}/{repo}/issues/{issue_number}/labels — Add labels to an issue
  b: labels:[] (one-of)

DELETE /repos/{owner}/{repo}/issues/{issue_number}/labels — Remove all labels from an issue ->204

DELETE /repos/{owner}/{repo}/issues/{issue_number}/labels/{name} — Remove a label from an issue

GET /repos/{owner}/{repo}/labels — List labels for a repository [pg]

POST /repos/{owner}/{repo}/labels — Create a label ->201
  b: name* color description

GET /repos/{owner}/{repo}/labels/{name} — Get a label

PATCH /repos/{owner}/{repo}/labels/{name} — Update a label
  b: color description new_name

DELETE /repos/{owner}/{repo}/labels/{name} — Delete a label ->204

GET /repos/{owner}/{repo}/milestones/{milestone_number}/labels — List labels for issues in a milestone [pg]
