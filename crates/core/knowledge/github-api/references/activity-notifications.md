# activity-notifications

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /notifications — List notifications for the authenticated user [pg]
  q: all:b=false participating:b=false since before

PUT /notifications — Mark notifications as read ->202
  b: last_read_at read:b

GET /notifications/threads/{thread_id} — Get a thread

PATCH /notifications/threads/{thread_id} — Mark a thread as read ->205

DELETE /notifications/threads/{thread_id} — Mark a thread as done ->204

GET /notifications/threads/{thread_id}/subscription — Get a thread subscription for the authenticated user

PUT /notifications/threads/{thread_id}/subscription — Set a thread subscription
  b: ignored:b

DELETE /notifications/threads/{thread_id}/subscription — Delete a thread subscription ->204

GET /repos/{owner}/{repo}/notifications — List repository notifications for the authenticated user [pg]
  q: all:b=false participating:b=false since before

PUT /repos/{owner}/{repo}/notifications — Mark repository notifications as read ->202
  b: last_read_at
