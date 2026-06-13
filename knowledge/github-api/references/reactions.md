# reactions

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/comments/{comment_id}/reactions — List reactions for a commit comment [pg]
  q: content(+1|-1|laugh|confused|heart|…)

POST /repos/{owner}/{repo}/comments/{comment_id}/reactions — Create reaction for a commit comment
  b: content*(+1|-1|laugh|confused|heart|…)

DELETE /repos/{owner}/{repo}/comments/{comment_id}/reactions/{reaction_id} — Delete a commit comment reaction ->204

GET /repos/{owner}/{repo}/issues/comments/{comment_id}/reactions — List reactions for an issue comment [pg]
  q: content(+1|-1|laugh|confused|heart|…)

POST /repos/{owner}/{repo}/issues/comments/{comment_id}/reactions — Create reaction for an issue comment
  b: content*(+1|-1|laugh|confused|heart|…)

DELETE /repos/{owner}/{repo}/issues/comments/{comment_id}/reactions/{reaction_id} — Delete an issue comment reaction ->204

GET /repos/{owner}/{repo}/issues/{issue_number}/reactions — List reactions for an issue [pg]
  q: content(+1|-1|laugh|confused|heart|…)

POST /repos/{owner}/{repo}/issues/{issue_number}/reactions — Create reaction for an issue
  b: content*(+1|-1|laugh|confused|heart|…)

DELETE /repos/{owner}/{repo}/issues/{issue_number}/reactions/{reaction_id} — Delete an issue reaction ->204

GET /repos/{owner}/{repo}/pulls/comments/{comment_id}/reactions — List reactions for a pull request review comment [pg]
  q: content(+1|-1|laugh|confused|heart|…)

POST /repos/{owner}/{repo}/pulls/comments/{comment_id}/reactions — Create reaction for a pull request review comment
  b: content*(+1|-1|laugh|confused|heart|…)

DELETE /repos/{owner}/{repo}/pulls/comments/{comment_id}/reactions/{reaction_id} — Delete a pull request comment reaction ->204

GET /repos/{owner}/{repo}/releases/{release_id}/reactions — List reactions for a release [pg]
  q: content(+1|laugh|heart|hooray|rocket|eyes)

POST /repos/{owner}/{repo}/releases/{release_id}/reactions — Create reaction for a release
  b: content*(+1|laugh|heart|hooray|rocket|eyes)

DELETE /repos/{owner}/{repo}/releases/{release_id}/reactions/{reaction_id} — Delete a release reaction ->204
