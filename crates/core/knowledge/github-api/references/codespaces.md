# codespaces

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/codespaces — List codespaces in a repository for the authenticated user [pg]

POST /repos/{owner}/{repo}/codespaces — Create a codespace in a repository ->201
  b: client_ip devcontainer_path display_name geo(EuropeWest|SoutheastAsia|UsEast|UsWest) idle_timeout_minutes:i location machine multi_repo_permissions_opt_out:b ref retention_period_minutes:i working_directory

GET /repos/{owner}/{repo}/codespaces/devcontainers — List devcontainer configurations in a repository for the authenticated user [pg]

GET /repos/{owner}/{repo}/codespaces/new — Get default attributes for a codespace
  q: ref client_ip

GET /repos/{owner}/{repo}/codespaces/permissions_check — Check if permissions defined by a devcontainer have been accepted by the authenticated user
  q: ref* devcontainer_path*

POST /repos/{owner}/{repo}/pulls/{pull_number}/codespaces — Create a codespace from a pull request ->201
  b: client_ip devcontainer_path display_name geo(EuropeWest|SoutheastAsia|UsEast|UsWest) idle_timeout_minutes:i location machine multi_repo_permissions_opt_out:b retention_period_minutes:i working_directory

GET /user/codespaces — List codespaces for the authenticated user [pg]
  q: repository_id:i

POST /user/codespaces — Create a codespace for the authenticated user ->201
  b: client_ip devcontainer_path display_name geo(EuropeWest|SoutheastAsia|UsEast|UsWest) idle_timeout_minutes:i location machine multi_repo_permissions_opt_out:b pull_request{pull_request_number*,repository_id*} ref repository_id:i retention_period_minutes:i working_directory (one-of)

GET /user/codespaces/{codespace_name} — Get a codespace for the authenticated user

PATCH /user/codespaces/{codespace_name} — Update a codespace for the authenticated user
  b: display_name machine recent_folders:[]

DELETE /user/codespaces/{codespace_name} — Delete a codespace for the authenticated user ->202

POST /user/codespaces/{codespace_name}/exports — Export a codespace for the authenticated user ->202

GET /user/codespaces/{codespace_name}/exports/{export_id} — Get details about a codespace export

POST /user/codespaces/{codespace_name}/publish — Create a repository from an unpublished codespace ->201
  b: name private:b

POST /user/codespaces/{codespace_name}/start — Start a codespace for the authenticated user

POST /user/codespaces/{codespace_name}/stop — Stop a codespace for the authenticated user
