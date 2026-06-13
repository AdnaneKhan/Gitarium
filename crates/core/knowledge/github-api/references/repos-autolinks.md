# repos-autolinks

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/autolinks — Get all autolinks of a repository

POST /repos/{owner}/{repo}/autolinks — Create an autolink reference for a repository ->201
  b: key_prefix* url_template* is_alphanumeric:b

GET /repos/{owner}/{repo}/autolinks/{autolink_id} — Get an autolink reference of a repository

DELETE /repos/{owner}/{repo}/autolinks/{autolink_id} — Delete an autolink reference from a repository ->204
