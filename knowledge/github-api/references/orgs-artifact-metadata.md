# orgs-artifact-metadata

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

POST /orgs/{org}/artifacts/metadata/deployment-record — Create an artifact deployment record
  b: deployment_name* digest* logical_environment* name* status*(deployed|decommissioned) cluster github_repository physical_environment return_records:b runtime_risks(critical-resource|internet-exposed|lateral-movement|sensitive-data) tags{} version

POST /orgs/{org}/artifacts/metadata/deployment-record/cluster/{cluster} — Set cluster deployment records
  b: deployments*:[o] logical_environment* physical_environment return_records:b

POST /orgs/{org}/artifacts/metadata/storage-record — Create artifact metadata storage record
  b: digest* name* registry_url* artifact_url github_repository path repository return_records:b status(active|eol|deleted) version

GET /orgs/{org}/artifacts/{subject_digest}/metadata/deployment-records — List artifact deployment records

GET /orgs/{org}/artifacts/{subject_digest}/metadata/storage-records — List artifact storage records
