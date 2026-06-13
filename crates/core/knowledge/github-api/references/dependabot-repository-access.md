# dependabot-repository-access

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /enterprises/{enterprise}/dependabot/repository-access — Lists the repositories Dependabot can access in an enterprise [pg]

PATCH /enterprises/{enterprise}/dependabot/repository-access — Updates Dependabot's repository access list for an enterprise ->204
  b: repository_ids_to_add:[i] repository_ids_to_remove:[i]

PUT /enterprises/{enterprise}/dependabot/repository-access/default-level — Set the default repository access level for Dependabot in an enterprise ->204
  b: default_level*(public|internal)

GET /orgs/{org}/dependabot/repository-access — Lists the repositories Dependabot can access in an organization [pg]

PATCH /orgs/{org}/dependabot/repository-access — Updates Dependabot's repository access list for an organization ->204
  b: repository_ids_to_add:[i] repository_ids_to_remove:[i]

PUT /orgs/{org}/dependabot/repository-access/default-level — Set the default repository access level for Dependabot ->204
  b: default_level*(public|internal)
