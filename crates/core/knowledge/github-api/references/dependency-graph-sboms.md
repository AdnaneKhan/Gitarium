# dependency-graph-sboms

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/dependency-graph/sbom — Export a software bill of materials (SBOM) for a repository.

GET /repos/{owner}/{repo}/dependency-graph/sbom/fetch-report/{sbom_uuid} — Fetch a software bill of materials (SBOM) for a repository. ->202

GET /repos/{owner}/{repo}/dependency-graph/sbom/generate-report — Request generation of a software bill of materials (SBOM) for a repository. ->201
