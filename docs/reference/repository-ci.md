---
role: reference
authority: normative
update-when: The GitHub Actions workflow path filters, repository-local action locations, workflow Conftest policy, pull request template sections, or PR description template check change.
---

# Repository CI

This page specifies repository-maintenance CI boundaries. It is not a source
for Veln language behavior.

## GitHub Actions

Repository-local composite actions live under `actions/`. Workflows that use a
local action must reference it from the repository action directory, with the
same relative action path shape used by workflow `uses` entries.

Path-filtered workflows that use a local action must list the exact
`actions/<name>/action.yaml` manifest in the matching trigger paths. Wildcard
local-action trigger paths are invalid because they can hide action manifest
changes from the workflow that consumes them.

The workflow Conftest policy enforces those local-action path-filter rules.

## Pull Request Template

Pull request descriptions use these H2 sections in this order:

1. `Intent`
2. `Consequences`
3. `Risks`
4. `Verification`

The PR description template check rejects missing, extra, or reordered H2
sections. The `Verification` section records concrete evidence for material
claims, such as named scenarios, assertions, fixtures, specification cases,
checks, or measurements. Passing commands alone are not sufficient evidence
when they do not identify the behavior or risk they validate.
