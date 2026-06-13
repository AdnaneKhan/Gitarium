# pulls-review-requests

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/pulls/{pull_number}/requested_reviewers — Get all requested reviewers for a pull request

POST /repos/{owner}/{repo}/pulls/{pull_number}/requested_reviewers — Request reviewers for a pull request ->201
  b: reviewers:[] team_reviewers:[]

DELETE /repos/{owner}/{repo}/pulls/{pull_number}/requested_reviewers — Remove requested reviewers from a pull request
  b: reviewers*:[] team_reviewers:[]
