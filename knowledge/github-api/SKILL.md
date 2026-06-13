---
name: github-api
description: Condensed GitHub REST API reference (X-GitHub-Api-Version 2026-03-10): every endpoint with query/body parameters, enums and defaults, grouped by topic. Consult before composing non-trivial API calls; see references/conventions.md for auth, pagination, rate limits, errors.
source: https://docs.github.com/en/rest?apiVersion=2026-03-10
openapi: github/rest-api-description v1.1.4
fetched: 2026-06-12
---

# GitHub REST API — condensed reference

Each references/ file lists endpoints as:

    METHOD /path — summary [flags]
      q: query params | b: body fields

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

Start with references/conventions.md (auth, versioning, pagination,
rate limits, media types, errors). Find an endpoint:
grep -i <keyword> /knowledge/github-api/references/

## Topics

- actions: actions-artifacts, actions-cache, actions-concurrency-groups, actions-hosted-runners, actions-oidc, actions-permissions, actions-secrets, actions-self-hosted-runner-groups, actions-self-hosted-runners, actions-variables, actions-workflow-jobs, actions-workflow-runs, actions-workflows
- activity: activity-events, activity-feeds, activity-notifications, activity-starring, activity-watching
- agent-tasks: agent-tasks
- agents: agents-secrets, agents-variables
- apps: apps, apps-installations, apps-marketplace, apps-oauth-applications, apps-webhooks
- billing: billing-budgets, billing-usage
- branches: branches, branches-branch-protection
- campaigns: campaigns
- checks: checks-runs, checks-suites
- classroom: classroom
- code-quality: code-quality
- code-scanning: code-scanning
- code-security: code-security-configurations
- codes-of-conduct: codes-of-conduct
- codespaces: codespaces, codespaces-machines, codespaces-organization-secrets, codespaces-organizations, codespaces-repository-secrets, codespaces-secrets
- collaborators: collaborators, collaborators-invitations
- commits: commits, commits-comments, commits-statuses
- copilot: copilot-cloud-agent-management, copilot-coding-agent-management, copilot-content-exclusion-management, copilot-metrics, copilot-usage-metrics, copilot-user-management
- copilot-spaces: copilot-spaces, copilot-spaces-collaborators, copilot-spaces-resources
- credentials: credentials-revoke
- dependabot: dependabot-alerts, dependabot-repository-access, dependabot-secrets
- dependency-graph: dependency-graph-dependency-review, dependency-graph-dependency-submission, dependency-graph-sboms
- deploy-keys: deploy-keys
- deployments: deployments, deployments-branch-policies, deployments-environments, deployments-protection-rules, deployments-statuses
- emojis: emojis
- enterprise-teams: enterprise-teams, enterprise-teams-enterprise-team-members, enterprise-teams-enterprise-team-organizations
- gists: gists, gists-comments
- git: git-blobs, git-commits, git-refs, git-tags, git-trees
- gitignore: gitignore
- interactions: interactions-orgs, interactions-repos, interactions-user
- issues: issues, issues-assignees, issues-comments, issues-events, issues-issue-dependencies, issues-issue-field-values, issues-labels, issues-milestones, issues-sub-issues, issues-timeline
- licenses: licenses
- markdown: markdown
- meta: meta
- metrics: metrics-community, metrics-statistics, metrics-traffic
- migrations: migrations-orgs, migrations-source-imports, migrations-users
- orgs: orgs, orgs-api-insights, orgs-artifact-metadata, orgs-attestations, orgs-blocking, orgs-custom-properties, orgs-issue-fields, orgs-issue-types, orgs-members, orgs-network-configurations, orgs-organization-roles, orgs-outside-collaborators, orgs-personal-access-tokens, orgs-rule-suites, orgs-rules, orgs-security-managers, orgs-webhooks
- packages: packages
- pages: pages
- private-registries: private-registries-organization-configurations
- projects: projects, projects-drafts, projects-fields, projects-items, projects-views
- pulls: pulls, pulls-comments, pulls-review-requests, pulls-reviews
- rate-limit: rate-limit
- reactions: reactions
- releases: releases, releases-assets
- repos: repos, repos-attestations, repos-autolinks, repos-contents, repos-custom-properties, repos-forks, repos-rule-suites, repos-rules, repos-webhooks
- search: search
- secret-scanning: secret-scanning, secret-scanning-push-protection
- security-advisories: security-advisories-global-advisories, security-advisories-repository-advisories
- teams: teams, teams-members
- users: users, users-attestations, users-blocking, users-emails, users-followers, users-gpg-keys, users-keys, users-social-accounts, users-ssh-signing-keys
