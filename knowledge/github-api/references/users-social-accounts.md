# users-social-accounts

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /user/social_accounts — List social accounts for the authenticated user [pg]

POST /user/social_accounts — Add social accounts for the authenticated user ->201
  b: account_urls*:[]

DELETE /user/social_accounts — Delete social accounts for the authenticated user ->204
  b: account_urls*:[]

GET /users/{username}/social_accounts — List social accounts for a user [pg]
