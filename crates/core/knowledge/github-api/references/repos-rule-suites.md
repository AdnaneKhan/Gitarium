# repos-rule-suites

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/rulesets/rule-suites — List repository rule suites [pg]
  q: ref time_period(hour|day|week|month)=day actor_name rule_suite_result(pass|fail|bypass|all)=all evaluate_status(all|active|evaluate)=all

GET /repos/{owner}/{repo}/rulesets/rule-suites/{rule_suite_id} — Get a repository rule suite
