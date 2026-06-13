# commits-statuses

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/commits/{ref}/status — Get the combined status for a specific reference [pg]

GET /repos/{owner}/{repo}/commits/{ref}/statuses — List commit statuses for a reference [pg]

POST /repos/{owner}/{repo}/statuses/{sha} — Create a commit status ->201
  b: state*(error|failure|pending|success) context description target_url
