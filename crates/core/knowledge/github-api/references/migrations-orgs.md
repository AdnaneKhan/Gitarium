# migrations-orgs

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/migrations — List organization migrations [pg]
  q: exclude(repositories)

POST /orgs/{org}/migrations — Start an organization migration ->201
  b: repositories*:[] exclude(repositories) exclude_attachments:b exclude_git_data:b exclude_metadata:b exclude_owner_projects:b exclude_releases:b lock_repositories:b org_metadata_only:b

GET /orgs/{org}/migrations/{migration_id} — Get an organization migration status
  q: exclude(repositories)

GET /orgs/{org}/migrations/{migration_id}/archive — Download an organization migration archive

DELETE /orgs/{org}/migrations/{migration_id}/archive — Delete an organization migration archive ->204

DELETE /orgs/{org}/migrations/{migration_id}/repos/{repo_name}/lock — Unlock an organization repository ->204

GET /orgs/{org}/migrations/{migration_id}/repositories — List repositories in an organization migration [pg]
