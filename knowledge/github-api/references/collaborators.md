# collaborators

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/collaborators — List repository collaborators [pg]
  q: affiliation(outside|direct|all)=all permission(pull|triage|push|maintain|admin)

GET /repos/{owner}/{repo}/collaborators/{username} — Check if a user is a repository collaborator ->204

PUT /repos/{owner}/{repo}/collaborators/{username} — Add a repository collaborator ->201
  b: permission

DELETE /repos/{owner}/{repo}/collaborators/{username} — Remove a repository collaborator ->204

GET /repos/{owner}/{repo}/collaborators/{username}/permission — Get repository permissions for a user
