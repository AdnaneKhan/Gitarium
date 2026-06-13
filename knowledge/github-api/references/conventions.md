# conventions

Cross-cutting rules for every GitHub REST call. Hand-maintained; not
regenerated from the OpenAPI description.

## Requests

- Base URL `https://api.github.com`. Always send:
  - `Accept: application/vnd.github+json`
  - `X-GitHub-Api-Version: 2026-03-10` (400 = version unsupported/retired)
  - `Authorization: Bearer <token>` when authenticated
- Timestamps are ISO 8601 UTC (`2026-06-12T00:00:00Z`) — both in responses
  and in `since`/`until` style parameters.
- Booleans in query strings are literal `true`/`false`; lists are usually
  comma-separated (`labels=bug,ui`).
- URL-encode path segments that may contain `/` or special chars (branch
  names, file paths in the contents API: encode each segment, keep `/`).

## Auth & permissions

- Fine-grained PATs need the specific repo + permission; classic PATs need
  scopes (`repo`, `workflow`, `admin:org`, ...).
- Missing permission on a private resource returns **404, not 403** — a 404
  on something that should exist usually means a token/scope problem.
- 403 with an `X-GitHub-SSO` header: token not authorized for an SSO org.
- Unauthenticated calls: 60 req/hr, public data only, writes fail.

## Pagination

- `per_page` max 100 (default 30). Follow the `Link` response header
  (`rel="next"`) or increment `page` while full pages keep coming.
- Some list endpoints are cursor-based: `after`/`before` params with
  `Link` headers only — no `page` numbers (e.g. audit log, some alerts).
- Search returns at most 1000 results total regardless of paging.
- `GET /repos/{o}/{r}/commits` pages back through history; use `sha` to
  start from a branch/commit and `path` to filter by file.

## Rate limits

- Authenticated core: 5000 req/hr. Search: 30 req/min (code search: 10).
  Check `x-ratelimit-remaining` / `x-ratelimit-reset` (epoch seconds) on
  every response; `GET /rate_limit` shows all buckets and is itself free.
- 403/429 with `Retry-After` = secondary limit: back off that many seconds.
  Avoid concurrent writes; pause ~1s between mutations to the same repo.
- Conditional requests: cache `ETag`, send `If-None-Match`; a 304 reply
  costs no rate limit and has no body.

## Media types (Accept header variants)

- `application/vnd.github.raw` — raw file bytes from the contents API.
- `application/vnd.github.diff` / `.patch` — diff/patch of a commit or PR.
- `application/vnd.github.html+json` — rendered HTML for markdown bodies.
- Default contents API response wraps files as base64 in JSON (`content`,
  `encoding` fields) — decode before use.

## Errors

- 401 bad credentials; 403 forbidden or rate-limited (check headers);
  404 not found *or* hidden by permissions; 409 conflict (e.g. empty
  repo on git-data endpoints, SHA mismatch on file updates).
- 422 validation failure — body has `message` plus `errors[]` of
  `{resource, field, code}`; `code: "custom"` puts detail in `message`.
- 301/302: repo was renamed/transferred — follow `Location` and update
  the stored full name.
- 202 Accepted: result is being computed (forks, repo stats, codespaces).
  Retry the GET after a short delay until 200.

## Gotchas

- `GET /repos/{o}/{r}/issues` returns **pull requests too** — filter out
  items that have a `pull_request` key when only issues are wanted.
- Contents API caps files at ~1 MB for reads (returns 403/`too_large`) —
  use `GET /repos/{o}/{r}/git/blobs/{sha}` (up to 100 MB) instead; find
  the blob sha via the tree endpoint (`git-trees`, `recursive=1`).
- Creating/updating a file (`PUT .../contents/{path}`) requires the
  current blob `sha` when the file exists; omitting it on update → 422/409.
- Release asset upload goes to `https://uploads.github.com`, not the API
  base: `POST .../releases/{id}/assets?name=...` with a real
  `Content-Type` and the raw bytes as body.
- Repo stats endpoints (`metrics-statistics`) return 202 on first call
  while GitHub computes them — poll.
- Checks vs statuses are different systems: `checks-runs`/`checks-suites`
  (Checks API, used by Actions) vs `commits-statuses` (legacy contexts);
  a "passing" commit may need both consulted (`status` + `check-runs`).
- Squash-merged PRs: `merge_commit_sha` is the squash commit; the head
  branch commits never appear on the base branch.
- `GET /search/*` needs `q` with qualifiers (`repo:`, `is:`, `language:`,
  `in:`); URL-encode the whole query string value.
- Git data writes (git-blobs/trees/commits/refs) compose bottom-up:
  blob → tree (with `base_tree` to keep existing files) → commit
  (`parents` = current head) → PATCH the ref. Forgetting `base_tree`
  silently drops every other file from the tree.
- Actions endpoints that dispatch (`workflow_dispatch`, re-runs) return
  204 with no body and no run id — list runs afterwards (filter by
  `event`, `created`, `head_sha`) to find the new run.
