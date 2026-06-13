# search

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /search/code — Search code [pg]
  q: q* sort(indexed) order(desc|asc)=desc

GET /search/commits — Search commits [pg]
  q: q* sort(author-date|committer-date) order(desc|asc)=desc

GET /search/issues — Search issues and pull requests [pg]
  q: q* sort(comments|reactions|reactions-+1|reactions--1|reactions-smile|…) order(desc|asc)=desc advanced_search search_type(semantic|hybrid)

GET /search/labels — Search labels [pg]
  q: repository_id*:i q* sort(created|updated) order(desc|asc)=desc

GET /search/repositories — Search repositories [pg]
  q: q* sort(stars|forks|help-wanted-issues|updated) order(desc|asc)=desc

GET /search/topics — Search topics [pg]
  q: q*

GET /search/users — Search users [pg]
  q: q* sort(followers|repositories|joined) order(desc|asc)=desc
