# users-emails

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

PATCH /user/email/visibility — Set primary email visibility for the authenticated user
  b: visibility*(public|private)

GET /user/emails — List email addresses for the authenticated user [pg]

POST /user/emails — Add an email address for the authenticated user ->201
  b: emails:[] (one-of)

DELETE /user/emails — Delete an email address for the authenticated user ->204
  b: emails:[] (one-of)

GET /user/public_emails — List public email addresses for the authenticated user [pg]
