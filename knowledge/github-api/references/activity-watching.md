# activity-watching

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/subscribers — List watchers [pg]

GET /repos/{owner}/{repo}/subscription — Get a repository subscription

PUT /repos/{owner}/{repo}/subscription — Set a repository subscription
  b: ignored:b subscribed:b

DELETE /repos/{owner}/{repo}/subscription — Delete a repository subscription ->204

GET /user/subscriptions — List repositories watched by the authenticated user [pg]

GET /users/{username}/subscriptions — List repositories watched by a user [pg]
