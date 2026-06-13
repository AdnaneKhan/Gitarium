# pages

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/pages — Get a GitHub Pages site

PUT /repos/{owner}/{repo}/pages — Update information about a GitHub Pages site ->204
  b: build_type(legacy|workflow) cname https_enforced:b source

POST /repos/{owner}/{repo}/pages — Create a GitHub Pages site ->201
  b: build_type(legacy|workflow) source{branch*}

DELETE /repos/{owner}/{repo}/pages — Delete a GitHub Pages site ->204

GET /repos/{owner}/{repo}/pages/builds — List GitHub Pages builds [pg]

POST /repos/{owner}/{repo}/pages/builds — Request a GitHub Pages build ->201

GET /repos/{owner}/{repo}/pages/builds/latest — Get latest Pages build

GET /repos/{owner}/{repo}/pages/builds/{build_id} — Get GitHub Pages build

POST /repos/{owner}/{repo}/pages/deployments — Create a GitHub Pages deployment
  b: oidc_token* pages_build_version* artifact_id:n artifact_url environment

GET /repos/{owner}/{repo}/pages/deployments/{pages_deployment_id} — Get the status of a GitHub Pages deployment

POST /repos/{owner}/{repo}/pages/deployments/{pages_deployment_id}/cancel — Cancel a GitHub Pages deployment ->204

GET /repos/{owner}/{repo}/pages/health — Get a DNS health check for GitHub Pages
