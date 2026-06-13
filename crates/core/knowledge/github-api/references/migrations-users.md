# migrations-users

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /user/migrations — List user migrations [pg]

POST /user/migrations — Start a user migration ->201
  b: repositories*:[] exclude(repositories) exclude_attachments:b exclude_git_data:b exclude_metadata:b exclude_owner_projects:b exclude_releases:b lock_repositories:b org_metadata_only:b

GET /user/migrations/{migration_id} — Get a user migration status
  q: exclude:[]

GET /user/migrations/{migration_id}/archive — Download a user migration archive

DELETE /user/migrations/{migration_id}/archive — Delete a user migration archive ->204

DELETE /user/migrations/{migration_id}/repos/{repo_name}/lock — Unlock a user repository ->204

GET /user/migrations/{migration_id}/repositories — List repositories for a user migration [pg]
