# pulls

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/pulls — List pull requests [pg]
  q: state(open|closed|all)=open head base sort(created|updated|popularity|long-running)=created direction(asc|desc)

POST /repos/{owner}/{repo}/pulls — Create a pull request ->201
  b: base* head* body draft:b head_repo issue:i maintainer_can_modify:b title

GET /repos/{owner}/{repo}/pulls/{pull_number} — Get a pull request

PATCH /repos/{owner}/{repo}/pulls/{pull_number} — Update a pull request
  b: base body maintainer_can_modify:b state(open|closed) title

GET /repos/{owner}/{repo}/pulls/{pull_number}/commits — List commits on a pull request [pg]

GET /repos/{owner}/{repo}/pulls/{pull_number}/files — List pull requests files [pg]

GET /repos/{owner}/{repo}/pulls/{pull_number}/merge — Check if a pull request has been merged ->204

PUT /repos/{owner}/{repo}/pulls/{pull_number}/merge — Merge a pull request
  b: commit_message commit_title merge_method(merge|squash|rebase) sha

PUT /repos/{owner}/{repo}/pulls/{pull_number}/update-branch — Update a pull request branch ->202
  b: expected_head_sha
