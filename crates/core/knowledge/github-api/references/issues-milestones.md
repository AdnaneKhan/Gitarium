# issues-milestones

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/milestones — List milestones [pg]
  q: state(open|closed|all)=open sort(due_on|completeness)=due_on direction(asc|desc)=asc

POST /repos/{owner}/{repo}/milestones — Create a milestone ->201
  b: title* description due_on state(open|closed)

GET /repos/{owner}/{repo}/milestones/{milestone_number} — Get a milestone

PATCH /repos/{owner}/{repo}/milestones/{milestone_number} — Update a milestone
  b: description due_on state(open|closed) title

DELETE /repos/{owner}/{repo}/milestones/{milestone_number} — Delete a milestone ->204
