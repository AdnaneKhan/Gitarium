# issues-events

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/issues/events — List issue events for a repository [pg]

GET /repos/{owner}/{repo}/issues/events/{event_id} — Get an issue event

GET /repos/{owner}/{repo}/issues/{issue_number}/events — List issue events [pg]
