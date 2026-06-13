# users

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /user — Get the authenticated user

PATCH /user — Update the authenticated user
  b: bio blog company email hireable:b location name twitter_username

GET /user/{account_id} — Get a user using their ID

GET /users — List users [pg]
  q: since:i

GET /users/{username} — Get a user

GET /users/{username}/hovercard — Get contextual information for a user
  q: subject_type(organization|repository|issue|pull_request) subject_id
