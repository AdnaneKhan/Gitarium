# dependency-graph-dependency-submission

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

POST /repos/{owner}/{repo}/dependency-graph/snapshots — Create a snapshot of dependencies for a repository ->201
  b: detector*{name*,version*,url*} job*{id*,correlator*} ref* scanned* sha* version*:i manifests{} metadata{}
