# repos-contents

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/contents/{path} — Get repository content
  q: ref

PUT /repos/{owner}/{repo}/contents/{path} — Create or update file contents
  b: content* message* author{name*,email*} branch committer{name*,email*} sha

DELETE /repos/{owner}/{repo}/contents/{path} — Delete a file
  b: message* sha* author{} branch committer{}

GET /repos/{owner}/{repo}/readme — Get a repository README
  q: ref

GET /repos/{owner}/{repo}/readme/{dir} — Get a repository README for a directory
  q: ref

GET /repos/{owner}/{repo}/tarball/{ref} — Download a repository archive (tar)

GET /repos/{owner}/{repo}/zipball/{ref} — Download a repository archive (zip)
