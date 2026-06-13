# repos-custom-properties

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/properties/values — Get all custom property values for a repository

PATCH /repos/{owner}/{repo}/properties/values — Create or update custom property values for a repository ->204
  b: properties*:[o]
