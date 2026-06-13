# activity-starring

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/stargazers — List stargazers [pg]

GET /user/starred — List repositories starred by the authenticated user [pg]
  q: sort(created|updated)=created direction(asc|desc)=desc

GET /user/starred/{owner}/{repo} — Check if a repository is starred by the authenticated user ->204

PUT /user/starred/{owner}/{repo} — Star a repository for the authenticated user ->204

DELETE /user/starred/{owner}/{repo} — Unstar a repository for the authenticated user ->204

GET /users/{username}/starred — List repositories starred by a user [pg]
  q: sort(created|updated)=created direction(asc|desc)=desc
