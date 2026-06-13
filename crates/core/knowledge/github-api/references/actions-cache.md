# actions-cache

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /enterprises/{enterprise}/actions/cache/retention-limit — Get GitHub Actions cache retention limit for an enterprise

PUT /enterprises/{enterprise}/actions/cache/retention-limit — Set GitHub Actions cache retention limit for an enterprise ->204
  b: max_cache_retention_days:i

GET /enterprises/{enterprise}/actions/cache/storage-limit — Get GitHub Actions cache storage limit for an enterprise

PUT /enterprises/{enterprise}/actions/cache/storage-limit — Set GitHub Actions cache storage limit for an enterprise ->204
  b: max_cache_size_gb:i

GET /organizations/{org}/actions/cache/retention-limit — Get GitHub Actions cache retention limit for an organization

PUT /organizations/{org}/actions/cache/retention-limit — Set GitHub Actions cache retention limit for an organization ->204
  b: max_cache_retention_days:i

GET /organizations/{org}/actions/cache/storage-limit — Get GitHub Actions cache storage limit for an organization

PUT /organizations/{org}/actions/cache/storage-limit — Set GitHub Actions cache storage limit for an organization ->204
  b: max_cache_size_gb:i

GET /orgs/{org}/actions/cache/usage — Get GitHub Actions cache usage for an organization

GET /orgs/{org}/actions/cache/usage-by-repository — List repositories with GitHub Actions cache usage for an organization [pg]

GET /repos/{owner}/{repo}/actions/cache/retention-limit — Get GitHub Actions cache retention limit for a repository

PUT /repos/{owner}/{repo}/actions/cache/retention-limit — Set GitHub Actions cache retention limit for a repository ->204
  b: max_cache_retention_days:i

GET /repos/{owner}/{repo}/actions/cache/storage-limit — Get GitHub Actions cache storage limit for a repository

PUT /repos/{owner}/{repo}/actions/cache/storage-limit — Set GitHub Actions cache storage limit for a repository ->204
  b: max_cache_size_gb:i

GET /repos/{owner}/{repo}/actions/cache/usage — Get GitHub Actions cache usage for a repository

GET /repos/{owner}/{repo}/actions/caches — List GitHub Actions caches for a repository [pg]
  q: ref key sort(created_at|last_accessed_at|size_in_bytes)=last_accessed_at direction(asc|desc)=desc

DELETE /repos/{owner}/{repo}/actions/caches — Delete GitHub Actions caches for a repository (using a cache key)
  q: key* ref

DELETE /repos/{owner}/{repo}/actions/caches/{cache_id} — Delete a GitHub Actions cache for a repository (using a cache ID) ->204
