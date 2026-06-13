# classroom

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /assignments/{assignment_id} — Get an assignment

GET /assignments/{assignment_id}/accepted_assignments — List accepted assignments for an assignment [pg]

GET /assignments/{assignment_id}/grades — Get assignment grades

GET /classrooms — List classrooms [pg]

GET /classrooms/{classroom_id} — Get a classroom

GET /classrooms/{classroom_id}/assignments — List assignments for a classroom [pg]
