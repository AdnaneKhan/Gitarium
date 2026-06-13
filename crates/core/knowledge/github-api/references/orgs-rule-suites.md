# orgs-rule-suites

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/rulesets/rule-suites — List organization rule suites [pg]
  q: ref repository_name time_period(hour|day|week|month)=day actor_name rule_suite_result(pass|fail|bypass|all)=all evaluate_status(all|active|evaluate)=all

GET /orgs/{org}/rulesets/rule-suites/{rule_suite_id} — Get an organization rule suite
