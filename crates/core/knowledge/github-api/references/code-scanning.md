# code-scanning

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/code-scanning/alerts — List code scanning alerts for an organization [pg]
  q: tool_name tool_guid before after direction(asc|desc)=desc state(open|closed|dismissed|fixed) sort(created|updated)=created severity(critical|high|medium|low|warning|…) assignees

GET /repos/{owner}/{repo}/code-scanning/alerts — List code scanning alerts for a repository [pg]
  q: tool_name tool_guid ref pr:i direction(asc|desc)=desc before after sort(created|updated)=created state(open|closed|dismissed|fixed) severity(critical|high|medium|low|warning|…) assignees

GET /repos/{owner}/{repo}/code-scanning/alerts/{alert_number} — Get a code scanning alert

PATCH /repos/{owner}/{repo}/code-scanning/alerts/{alert_number} — Update a code scanning alert
  b: assignees:[] create_request:b dismissed_comment dismissed_reason(false positive|won't fix|used in tests) state(open|dismissed)

GET /repos/{owner}/{repo}/code-scanning/alerts/{alert_number}/autofix — Get the status of an autofix for a code scanning alert

POST /repos/{owner}/{repo}/code-scanning/alerts/{alert_number}/autofix — Create an autofix for a code scanning alert

POST /repos/{owner}/{repo}/code-scanning/alerts/{alert_number}/autofix/commits — Commit an autofix for a code scanning alert ->201
  b: message target_ref

GET /repos/{owner}/{repo}/code-scanning/alerts/{alert_number}/instances — List instances of a code scanning alert [pg]
  q: ref pr:i

GET /repos/{owner}/{repo}/code-scanning/analyses — List code scanning analyses for a repository [pg]
  q: tool_name tool_guid pr:i ref sarif_id direction(asc|desc)=desc sort(created)=created

GET /repos/{owner}/{repo}/code-scanning/analyses/{analysis_id} — Get a code scanning analysis for a repository

DELETE /repos/{owner}/{repo}/code-scanning/analyses/{analysis_id} — Delete a code scanning analysis from a repository
  q: confirm_delete

GET /repos/{owner}/{repo}/code-scanning/codeql/databases — List CodeQL databases for a repository

GET /repos/{owner}/{repo}/code-scanning/codeql/databases/{language} — Get a CodeQL database for a repository

DELETE /repos/{owner}/{repo}/code-scanning/codeql/databases/{language} — Delete a CodeQL database ->204

POST /repos/{owner}/{repo}/code-scanning/codeql/variant-analyses — Create a CodeQL variant analysis ->201
  b: language*(actions|cpp|csharp|go|java|…) query_pack* repositories:[] repository_lists:[] repository_owners:[]

GET /repos/{owner}/{repo}/code-scanning/codeql/variant-analyses/{codeql_variant_analysis_id} — Get the summary of a CodeQL variant analysis

GET /repos/{owner}/{repo}/code-scanning/codeql/variant-analyses/{codeql_variant_analysis_id}/repos/{repo_owner}/{repo_name} — Get the analysis status of a repository in a CodeQL variant analysis

GET /repos/{owner}/{repo}/code-scanning/default-setup — Get a code scanning default setup configuration

PATCH /repos/{owner}/{repo}/code-scanning/default-setup — Update a code scanning default setup configuration
  b: languages(actions|c-cpp|csharp|go|java-kotlin|…) query_suite(default|extended) runner_label runner_type(standard|labeled) state(configured|not-configured) threat_model(remote|remote_and_local)

POST /repos/{owner}/{repo}/code-scanning/sarifs — Upload an analysis as SARIF data ->202
  b: commit_sha* ref* sarif* checkout_uri started_at tool_name validate:b

GET /repos/{owner}/{repo}/code-scanning/sarifs/{sarif_id} — Get information about a SARIF upload
