# orgs-personal-access-tokens

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/personal-access-token-requests — List requests to access organization resources with fine-grained personal access tokens [pg]
  q: sort(created_at)=created_at direction(asc|desc)=desc owner:[] repository permission last_used_before last_used_after token_id:[]

POST /orgs/{org}/personal-access-token-requests — Review requests to access organization resources with fine-grained personal access tokens ->202
  b: action*(approve|deny) pat_request_ids:[i] reason

POST /orgs/{org}/personal-access-token-requests/{pat_request_id} — Review a request to access organization resources with a fine-grained personal access token ->204
  b: action*(approve|deny) reason

GET /orgs/{org}/personal-access-token-requests/{pat_request_id}/repositories — List repositories requested to be accessed by a fine-grained personal access token [pg]

GET /orgs/{org}/personal-access-tokens — List fine-grained personal access tokens with access to organization resources [pg]
  q: sort(created_at)=created_at direction(asc|desc)=desc owner:[] repository permission last_used_before last_used_after token_id:[]

POST /orgs/{org}/personal-access-tokens — Update the access to organization resources via fine-grained personal access tokens ->202
  b: action*(revoke) pat_ids*:[i]

POST /orgs/{org}/personal-access-tokens/{pat_id} — Update the access a fine-grained personal access token has to organization resources ->204
  b: action*(revoke)

GET /orgs/{org}/personal-access-tokens/{pat_id}/repositories — List repositories a fine-grained personal access token has access to [pg]
