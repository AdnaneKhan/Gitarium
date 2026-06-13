# collaborators-invitations

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/invitations — List repository invitations [pg]

PATCH /repos/{owner}/{repo}/invitations/{invitation_id} — Update a repository invitation
  b: permissions(read|write|maintain|triage|admin)

DELETE /repos/{owner}/{repo}/invitations/{invitation_id} — Delete a repository invitation ->204

GET /user/repository_invitations — List repository invitations for the authenticated user [pg]

PATCH /user/repository_invitations/{invitation_id} — Accept a repository invitation ->204

DELETE /user/repository_invitations/{invitation_id} — Decline a repository invitation ->204
