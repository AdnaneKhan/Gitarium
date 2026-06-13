# agent-tasks

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /agents/repos/{owner}/{repo}/tasks — List tasks for repository [pg]
  q: sort(updated_at|created_at)=updated_at direction(asc|desc)=desc state is_archived:b=false since creator_id:[i]

POST /agents/repos/{owner}/{repo}/tasks — Start a task ->201
  b: prompt* base_ref create_pull_request:b head_ref model

GET /agents/repos/{owner}/{repo}/tasks/{task_id} — Get a task by repo

GET /agents/tasks — List tasks [pg]
  q: sort(updated_at|created_at)=updated_at direction(asc|desc)=desc state is_archived:b=false since

GET /agents/tasks/{task_id} — Get a task by ID
