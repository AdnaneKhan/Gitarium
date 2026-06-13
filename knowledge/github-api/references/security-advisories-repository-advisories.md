# security-advisories-repository-advisories

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/security-advisories — List repository security advisories for an organization [pg]
  q: direction(asc|desc)=desc sort(created|updated|published)=created before after state(triage|draft|published|closed)

GET /repos/{owner}/{repo}/security-advisories — List repository security advisories [pg]
  q: direction(asc|desc)=desc sort(created|updated|published)=created before after state(triage|draft|published|closed)

POST /repos/{owner}/{repo}/security-advisories — Create a repository security advisory ->201
  b: description* summary* vulnerabilities*:[o] credits:[o] cve_id cvss_vector_string cwe_ids:[] severity(critical|high|medium|low) start_private_fork:b

POST /repos/{owner}/{repo}/security-advisories/reports — Privately report a security vulnerability ->201
  b: description* summary* cvss_vector_string cwe_ids:[] severity(critical|high|medium|low) start_private_fork:b vulnerabilities:[o]

GET /repos/{owner}/{repo}/security-advisories/{ghsa_id} — Get a repository security advisory

PATCH /repos/{owner}/{repo}/security-advisories/{ghsa_id} — Update a repository security advisory
  b: collaborating_teams:[] collaborating_users:[] credits:[o] cve_id cvss_vector_string cwe_ids:[] description severity(critical|high|medium|low) state(published|closed|draft) summary vulnerabilities:[o]

POST /repos/{owner}/{repo}/security-advisories/{ghsa_id}/cve — Request a CVE for a repository security advisory ->202

POST /repos/{owner}/{repo}/security-advisories/{ghsa_id}/forks — Create a temporary private fork ->202
