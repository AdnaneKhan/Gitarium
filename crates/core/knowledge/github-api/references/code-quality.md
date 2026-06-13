# code-quality

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/code-quality/setup — Get a code quality setup configuration

PATCH /repos/{owner}/{repo}/code-quality/setup — Update a code quality setup configuration
  b: languages(csharp|go|java-kotlin|javascript-typescript|python|ruby) runner_label runner_type(standard|labeled) state(configured|not-configured)
