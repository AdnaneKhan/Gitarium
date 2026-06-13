# secret-scanning-push-protection

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /orgs/{org}/secret-scanning/pattern-configurations — List organization pattern configurations

PATCH /orgs/{org}/secret-scanning/pattern-configurations — Update organization pattern configurations
  b: custom_pattern_settings:[o] pattern_config_version provider_pattern_settings:[o]
