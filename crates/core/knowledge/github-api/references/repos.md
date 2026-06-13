# repos

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/repos — List organization repositories [pg]
  q: type(all|public|private|forks|sources|member)=all sort(created|updated|pushed|full_name)=created direction(asc|desc)

POST /orgs/{org}/repos — Create an organization repository ->201
  b: name* allow_auto_merge:b allow_merge_commit:b allow_rebase_merge:b allow_squash_merge:b auto_init:b custom_properties{} delete_branch_on_merge:b description gitignore_template has_downloads:b has_issues:b has_projects:b has_wiki:b homepage is_template:b license_template merge_commit_message(PR_BODY|PR_TITLE|BLANK) merge_commit_title(PR_TITLE|MERGE_MESSAGE) private:b squash_merge_commit_message(PR_BODY|COMMIT_MESSAGES|BLANK) squash_merge_commit_title(PR_TITLE|COMMIT_OR_PR_TITLE) team_id:i use_squash_pr_title_as_default:b visibility(public|private)

GET /repos/{owner}/{repo} — Get a repository

PATCH /repos/{owner}/{repo} — Update a repository
  b: allow_auto_merge:b allow_forking:b allow_merge_commit:b allow_rebase_merge:b allow_squash_merge:b allow_update_branch:b archived:b default_branch delete_branch_on_merge:b description has_issues:b has_projects:b has_pull_requests:b has_wiki:b homepage is_template:b merge_commit_message(PR_BODY|PR_TITLE|BLANK) merge_commit_title(PR_TITLE|MERGE_MESSAGE) name private:b pull_request_creation_policy(all|collaborators_only) security_and_analysis{} squash_merge_commit_message(PR_BODY|COMMIT_MESSAGES|BLANK) squash_merge_commit_title(PR_TITLE|COMMIT_OR_PR_TITLE) use_squash_pr_title_as_default:b visibility(public|private) web_commit_signoff_required:b

DELETE /repos/{owner}/{repo} — Delete a repository ->204

GET /repos/{owner}/{repo}/activity — List repository activities [pg]
  q: direction(asc|desc)=desc before after ref actor time_period(day|week|month|quarter|year) activity_type(push|force_push|branch_creation|branch_deletion|pr_merge|merge_queue_merge)

GET /repos/{owner}/{repo}/automated-security-fixes — Check if Dependabot security updates are enabled for a repository

PUT /repos/{owner}/{repo}/automated-security-fixes — Enable Dependabot security updates ->204

DELETE /repos/{owner}/{repo}/automated-security-fixes — Disable Dependabot security updates ->204

GET /repos/{owner}/{repo}/codeowners/errors — List CODEOWNERS errors
  q: ref

GET /repos/{owner}/{repo}/contributors — List repository contributors [pg]
  q: anon

POST /repos/{owner}/{repo}/dispatches — Create a repository dispatch event ->204
  b: event_type* client_payload{}

GET /repos/{owner}/{repo}/hash-algorithm — Get the hash algorithm for a repository

GET /repos/{owner}/{repo}/immutable-releases — Check if immutable releases are enabled for a repository

PUT /repos/{owner}/{repo}/immutable-releases — Enable immutable releases ->204

DELETE /repos/{owner}/{repo}/immutable-releases — Disable immutable releases ->204

GET /repos/{owner}/{repo}/languages — List repository languages

GET /repos/{owner}/{repo}/private-vulnerability-reporting — Check if private vulnerability reporting is enabled for a repository

PUT /repos/{owner}/{repo}/private-vulnerability-reporting — Enable private vulnerability reporting for a repository ->204

DELETE /repos/{owner}/{repo}/private-vulnerability-reporting — Disable private vulnerability reporting for a repository ->204

GET /repos/{owner}/{repo}/tags — List repository tags [pg]

GET /repos/{owner}/{repo}/teams — List repository teams [pg]

GET /repos/{owner}/{repo}/topics — Get all repository topics [pg]

PUT /repos/{owner}/{repo}/topics — Replace all repository topics
  b: names*:[]

POST /repos/{owner}/{repo}/transfer — Transfer a repository ->202
  b: new_owner* new_name team_ids:[i]

GET /repos/{owner}/{repo}/vulnerability-alerts — Check if vulnerability alerts are enabled for a repository ->204

PUT /repos/{owner}/{repo}/vulnerability-alerts — Enable vulnerability alerts ->204

DELETE /repos/{owner}/{repo}/vulnerability-alerts — Disable vulnerability alerts ->204

POST /repos/{template_owner}/{template_repo}/generate — Create a repository using a template ->201
  b: name* description include_all_branches:b owner private:b

GET /repositories — List public repositories
  q: since:i

GET /user/repos — List repositories for the authenticated user [pg]
  q: visibility(all|public|private)=all affiliation=owner,collaborator,organization_member type(all|owner|public|private|member)=all sort(created|updated|pushed|full_name)=full_name direction(asc|desc) since before

POST /user/repos — Create a repository for the authenticated user ->201
  b: name* allow_auto_merge:b allow_merge_commit:b allow_rebase_merge:b allow_squash_merge:b auto_init:b delete_branch_on_merge:b description gitignore_template has_discussions:b has_downloads:b has_issues:b has_projects:b has_wiki:b homepage is_template:b license_template merge_commit_message(PR_BODY|PR_TITLE|BLANK) merge_commit_title(PR_TITLE|MERGE_MESSAGE) private:b squash_merge_commit_message(PR_BODY|COMMIT_MESSAGES|BLANK) squash_merge_commit_title(PR_TITLE|COMMIT_OR_PR_TITLE) team_id:i

GET /users/{username}/repos — List repositories for a user [pg]
  q: type(all|owner|member)=owner sort(created|updated|pushed|full_name)=full_name direction(asc|desc)
