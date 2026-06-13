# repos-attestations

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

POST /repos/{owner}/{repo}/attestations — Create an attestation ->201
  b: bundle*{}

GET /repos/{owner}/{repo}/attestations/{subject_digest} — List attestations [pg]
  q: before after predicate_type
