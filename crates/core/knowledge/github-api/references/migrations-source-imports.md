# migrations-source-imports

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/import — Get an import status (deprecated)

PUT /repos/{owner}/{repo}/import — Start an import ->201 (deprecated)
  b: vcs_url* tfvc_project vcs(subversion|git|mercurial|tfvc) vcs_password vcs_username

PATCH /repos/{owner}/{repo}/import — Update an import (deprecated)
  b: tfvc_project vcs(subversion|tfvc|git|mercurial) vcs_password vcs_username

DELETE /repos/{owner}/{repo}/import — Cancel an import ->204 (deprecated)

GET /repos/{owner}/{repo}/import/authors — Get commit authors (deprecated)
  q: since:i

PATCH /repos/{owner}/{repo}/import/authors/{author_id} — Map a commit author (deprecated)
  b: email name

GET /repos/{owner}/{repo}/import/large_files — Get large files (deprecated)

PATCH /repos/{owner}/{repo}/import/lfs — Update Git LFS preference (deprecated)
  b: use_lfs*(opt_in|opt_out)
