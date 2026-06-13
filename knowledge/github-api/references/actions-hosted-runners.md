# actions-hosted-runners

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/actions/hosted-runners — List GitHub-hosted runners for an organization [pg]

POST /orgs/{org}/actions/hosted-runners — Create a GitHub-hosted runner for an organization ->201
  b: image*{} name* runner_group_id*:i size* enable_static_ip:b image_gen:b maximum_runners:i

GET /orgs/{org}/actions/hosted-runners/images/custom — List custom images for an organization

GET /orgs/{org}/actions/hosted-runners/images/custom/{image_definition_id} — Get a custom image definition for GitHub Actions Hosted Runners

DELETE /orgs/{org}/actions/hosted-runners/images/custom/{image_definition_id} — Delete a custom image from the organization ->204

GET /orgs/{org}/actions/hosted-runners/images/custom/{image_definition_id}/versions — List image versions of a custom image for an organization

GET /orgs/{org}/actions/hosted-runners/images/custom/{image_definition_id}/versions/{version} — Get an image version of a custom image for GitHub Actions Hosted Runners

DELETE /orgs/{org}/actions/hosted-runners/images/custom/{image_definition_id}/versions/{version} — Delete an image version of custom image from the organization ->204

GET /orgs/{org}/actions/hosted-runners/images/github-owned — Get GitHub-owned images for GitHub-hosted runners in an organization

GET /orgs/{org}/actions/hosted-runners/images/partner — Get partner images for GitHub-hosted runners in an organization

GET /orgs/{org}/actions/hosted-runners/limits — Get limits on GitHub-hosted runners for an organization

GET /orgs/{org}/actions/hosted-runners/machine-sizes — Get GitHub-hosted runners machine specs for an organization

GET /orgs/{org}/actions/hosted-runners/platforms — Get platforms for GitHub-hosted runners in an organization

GET /orgs/{org}/actions/hosted-runners/{hosted_runner_id} — Get a GitHub-hosted runner for an organization

PATCH /orgs/{org}/actions/hosted-runners/{hosted_runner_id} — Update a GitHub-hosted runner for an organization
  b: enable_static_ip:b image_gen:b image_id image_source(github|partner|custom) image_version maximum_runners:i name runner_group_id:i size

DELETE /orgs/{org}/actions/hosted-runners/{hosted_runner_id} — Delete a GitHub-hosted runner for an organization ->202
