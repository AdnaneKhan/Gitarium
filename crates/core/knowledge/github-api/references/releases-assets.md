# releases-assets

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/releases/assets/{asset_id} — Get a release asset

PATCH /repos/{owner}/{repo}/releases/assets/{asset_id} — Update a release asset
  b: label name state

DELETE /repos/{owner}/{repo}/releases/assets/{asset_id} — Delete a release asset ->204

GET /repos/{owner}/{repo}/releases/{release_id}/assets — List release assets [pg]

POST /repos/{owner}/{repo}/releases/{release_id}/assets — Upload a release asset ->201
  q: name* label
  b: raw (application/octet-stream)
