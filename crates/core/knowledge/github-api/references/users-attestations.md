# users-attestations

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

POST /users/{username}/attestations/bulk-list — List attestations by bulk subject digests [pg]
  q: before after
  b: subject_digests*:[] predicate_type

POST /users/{username}/attestations/delete-request — Delete attestations in bulk
  b: attestation_ids:[i] subject_digests:[] (one-of)

DELETE /users/{username}/attestations/digest/{subject_digest} — Delete attestations by subject digest

DELETE /users/{username}/attestations/{attestation_id} — Delete attestations by ID

GET /users/{username}/attestations/{subject_digest} — List attestations [pg]
  q: before after predicate_type
