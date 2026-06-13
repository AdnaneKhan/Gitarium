# orgs-api-insights

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/insights/api/route-stats/{actor_type}/{actor_id} — Get route stats by actor [pg]
  q: min_timestamp* max_timestamp direction(asc|desc)=desc sort(last_rate_limited_timestamp|last_request_timestamp|rate_limited_request_count|http_method|api_route|total_request_count) api_route_substring

GET /orgs/{org}/insights/api/subject-stats — Get subject stats [pg]
  q: min_timestamp* max_timestamp direction(asc|desc)=desc sort(last_rate_limited_timestamp|last_request_timestamp|rate_limited_request_count|subject_name|total_request_count) subject_name_substring

GET /orgs/{org}/insights/api/summary-stats — Get summary stats
  q: min_timestamp* max_timestamp

GET /orgs/{org}/insights/api/summary-stats/users/{user_id} — Get summary stats by user
  q: min_timestamp* max_timestamp

GET /orgs/{org}/insights/api/summary-stats/{actor_type}/{actor_id} — Get summary stats by actor
  q: min_timestamp* max_timestamp

GET /orgs/{org}/insights/api/time-stats — Get time stats
  q: min_timestamp* max_timestamp timestamp_increment*

GET /orgs/{org}/insights/api/time-stats/users/{user_id} — Get time stats by user
  q: min_timestamp* max_timestamp timestamp_increment*

GET /orgs/{org}/insights/api/time-stats/{actor_type}/{actor_id} — Get time stats by actor
  q: min_timestamp* max_timestamp timestamp_increment*

GET /orgs/{org}/insights/api/user-stats/{user_id} — Get user stats [pg]
  q: min_timestamp* max_timestamp direction(asc|desc)=desc sort(last_rate_limited_timestamp|last_request_timestamp|rate_limited_request_count|subject_name|total_request_count) actor_name_substring
