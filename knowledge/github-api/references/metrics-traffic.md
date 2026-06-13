# metrics-traffic

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /repos/{owner}/{repo}/traffic/clones — Get repository clones
  q: per(day|week)=day

GET /repos/{owner}/{repo}/traffic/popular/paths — Get top referral paths

GET /repos/{owner}/{repo}/traffic/popular/referrers — Get top referral sources

GET /repos/{owner}/{repo}/traffic/views — Get page views
  q: per(day|week)=day
