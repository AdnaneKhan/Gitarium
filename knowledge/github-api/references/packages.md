# packages

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/docker/conflicts — Get list of conflicting packages during Docker migration for organization

GET /orgs/{org}/packages — List packages for an organization [pg]
  q: package_type*(npm|maven|rubygems|docker|nuget|container) visibility(public|private|internal)

GET /orgs/{org}/packages/{package_type}/{package_name} — Get a package for an organization

DELETE /orgs/{org}/packages/{package_type}/{package_name} — Delete a package for an organization ->204

POST /orgs/{org}/packages/{package_type}/{package_name}/restore — Restore a package for an organization ->204
  q: token

GET /orgs/{org}/packages/{package_type}/{package_name}/versions — List package versions for a package owned by an organization [pg]
  q: state(active|deleted)=active

GET /orgs/{org}/packages/{package_type}/{package_name}/versions/{package_version_id} — Get a package version for an organization

DELETE /orgs/{org}/packages/{package_type}/{package_name}/versions/{package_version_id} — Delete package version for an organization ->204

POST /orgs/{org}/packages/{package_type}/{package_name}/versions/{package_version_id}/restore — Restore package version for an organization ->204

GET /user/docker/conflicts — Get list of conflicting packages during Docker migration for authenticated-user

GET /user/packages — List packages for the authenticated user's namespace [pg]
  q: package_type*(npm|maven|rubygems|docker|nuget|container) visibility(public|private|internal)

GET /user/packages/{package_type}/{package_name} — Get a package for the authenticated user

DELETE /user/packages/{package_type}/{package_name} — Delete a package for the authenticated user ->204

POST /user/packages/{package_type}/{package_name}/restore — Restore a package for the authenticated user ->204
  q: token

GET /user/packages/{package_type}/{package_name}/versions — List package versions for a package owned by the authenticated user [pg]
  q: state(active|deleted)=active

GET /user/packages/{package_type}/{package_name}/versions/{package_version_id} — Get a package version for the authenticated user

DELETE /user/packages/{package_type}/{package_name}/versions/{package_version_id} — Delete a package version for the authenticated user ->204

POST /user/packages/{package_type}/{package_name}/versions/{package_version_id}/restore — Restore a package version for the authenticated user ->204

GET /users/{username}/docker/conflicts — Get list of conflicting packages during Docker migration for user

GET /users/{username}/packages — List packages for a user [pg]
  q: package_type*(npm|maven|rubygems|docker|nuget|container) visibility(public|private|internal)

GET /users/{username}/packages/{package_type}/{package_name} — Get a package for a user

DELETE /users/{username}/packages/{package_type}/{package_name} — Delete a package for a user ->204

POST /users/{username}/packages/{package_type}/{package_name}/restore — Restore a package for a user ->204
  q: token

GET /users/{username}/packages/{package_type}/{package_name}/versions — List package versions for a package owned by a user

GET /users/{username}/packages/{package_type}/{package_name}/versions/{package_version_id} — Get a package version for a user

DELETE /users/{username}/packages/{package_type}/{package_name}/versions/{package_version_id} — Delete package version for a user ->204

POST /users/{username}/packages/{package_type}/{package_name}/versions/{package_version_id}/restore — Restore package version for a user ->204
