# security-advisories-global-advisories

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

GET /advisories — List global security advisories [pg]
  q: ghsa_id type(reviewed|malware|unreviewed)=reviewed cve_id ecosystem(rubygems|npm|pip|maven|nuget|…) severity(unknown|low|medium|high|critical) cwes is_withdrawn:b affects published updated modified epss_percentage epss_percentile before after direction(asc|desc)=desc sort(updated|published|epss_percentage|epss_percentile)=published

GET /advisories/{ghsa_id} — Get a global security advisory
