# pulls-reviews

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/pulls/{pull_number}/reviews — List reviews for a pull request [pg]

POST /repos/{owner}/{repo}/pulls/{pull_number}/reviews — Create a review for a pull request
  b: body comments:[o] commit_id event(APPROVE|REQUEST_CHANGES|COMMENT)

GET /repos/{owner}/{repo}/pulls/{pull_number}/reviews/{review_id} — Get a review for a pull request

PUT /repos/{owner}/{repo}/pulls/{pull_number}/reviews/{review_id} — Update a review for a pull request
  b: body*

DELETE /repos/{owner}/{repo}/pulls/{pull_number}/reviews/{review_id} — Delete a pending review for a pull request

GET /repos/{owner}/{repo}/pulls/{pull_number}/reviews/{review_id}/comments — List comments for a pull request review [pg]

PUT /repos/{owner}/{repo}/pulls/{pull_number}/reviews/{review_id}/dismissals — Dismiss a review for a pull request
  b: message* event(DISMISS)

POST /repos/{owner}/{repo}/pulls/{pull_number}/reviews/{review_id}/events — Submit a review for a pull request
  b: event*(APPROVE|REQUEST_CHANGES|COMMENT) body
