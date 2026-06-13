# checks-runs

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

POST /repos/{owner}/{repo}/check-runs — Create a check run ->201
  b: head_sha* name* actions:[o] completed_at conclusion(action_required|cancelled|failure|neutral|success|…) details_url external_id output{title*,summary*} started_at status(queued|in_progress|completed|waiting|requested|pending)

GET /repos/{owner}/{repo}/check-runs/{check_run_id} — Get a check run

PATCH /repos/{owner}/{repo}/check-runs/{check_run_id} — Update a check run
  b: actions:[o] completed_at conclusion(action_required|cancelled|failure|neutral|success|…) details_url external_id name output{summary*} started_at status(queued|in_progress|completed|waiting|requested|pending)

GET /repos/{owner}/{repo}/check-runs/{check_run_id}/annotations — List check run annotations [pg]

POST /repos/{owner}/{repo}/check-runs/{check_run_id}/rerequest — Rerequest a check run ->201

GET /repos/{owner}/{repo}/check-suites/{check_suite_id}/check-runs — List check runs in a check suite [pg]
  q: check_name status(queued|in_progress|completed) filter(latest|all)=latest

GET /repos/{owner}/{repo}/commits/{ref}/check-runs — List check runs for a Git reference [pg]
  q: check_name status(queued|in_progress|completed) filter(latest|all)=latest app_id:i
