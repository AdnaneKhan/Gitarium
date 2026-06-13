# users-followers

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /user/followers — List followers of the authenticated user [pg]

GET /user/following — List the people the authenticated user follows [pg]

GET /user/following/{username} — Check if a person is followed by the authenticated user ->204

PUT /user/following/{username} — Follow a user ->204

DELETE /user/following/{username} — Unfollow a user ->204

GET /users/{username}/followers — List followers of a user [pg]

GET /users/{username}/following — List the people a user follows [pg]

GET /users/{username}/following/{target_user} — Check if a user follows another user ->204
