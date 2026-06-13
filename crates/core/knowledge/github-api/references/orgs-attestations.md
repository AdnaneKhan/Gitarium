# orgs-attestations

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

POST /orgs/{org}/attestations/bulk-list — List attestations by bulk subject digests [pg]
  q: before after
  b: subject_digests*:[] predicate_type

POST /orgs/{org}/attestations/delete-request — Delete attestations in bulk
  b: attestation_ids:[i] subject_digests:[] (one-of)

DELETE /orgs/{org}/attestations/digest/{subject_digest} — Delete attestations by subject digest

GET /orgs/{org}/attestations/repositories — List attestation repositories [pg]
  q: before after predicate_type

DELETE /orgs/{org}/attestations/{attestation_id} — Delete attestations by ID

GET /orgs/{org}/attestations/{subject_digest} — List attestations [pg]
  q: before after predicate_type
