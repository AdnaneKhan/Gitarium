# git-tags

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

POST /repos/{owner}/{repo}/git/tags — Create a tag object ->201
  b: message* object* tag* type*(commit|tree|blob) tagger{name*,email*}

GET /repos/{owner}/{repo}/git/tags/{tag_sha} — Get a tag
