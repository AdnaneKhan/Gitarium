# activity-events

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /events — List public events [pg]

GET /networks/{owner}/{repo}/events — List public events for a network of repositories [pg]

GET /orgs/{org}/events — List public organization events [pg]

GET /repos/{owner}/{repo}/events — List repository events [pg]

GET /users/{username}/events — List events for the authenticated user [pg]

GET /users/{username}/events/orgs/{org} — List organization events for the authenticated user [pg]

GET /users/{username}/events/public — List public events for a user [pg]

GET /users/{username}/received_events — List events received by the authenticated user [pg]

GET /users/{username}/received_events/public — List public events received by a user [pg]
