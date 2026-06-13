# releases

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/releases — List releases [pg]

POST /repos/{owner}/{repo}/releases — Create a release ->201
  b: tag_name* body discussion_category_name draft:b generate_release_notes:b make_latest(true|false|legacy) name prerelease:b target_commitish

POST /repos/{owner}/{repo}/releases/generate-notes — Generate release notes content for a release
  b: tag_name* configuration_file_path previous_tag_name target_commitish

GET /repos/{owner}/{repo}/releases/latest — Get the latest release

GET /repos/{owner}/{repo}/releases/tags/{tag} — Get a release by tag name

GET /repos/{owner}/{repo}/releases/{release_id} — Get a release

PATCH /repos/{owner}/{repo}/releases/{release_id} — Update a release
  b: body discussion_category_name draft:b make_latest(true|false|legacy) name prerelease:b tag_name target_commitish

DELETE /repos/{owner}/{repo}/releases/{release_id} — Delete a release ->204
