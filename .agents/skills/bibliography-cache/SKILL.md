---
name: bibliography-cache
description: Create, update, or validate the repository's cached bibliography records and search results.
---

# Bibliography Cache

## Goal

Keep reference metadata reusable, auditable, and cheap to retrieve without
turning a working cache into project authority.

Use `.bibliography-cache/` in the current project unless the user specifies a
different cache root. Create only the records needed for the task.

## Project Governance

The cache is a working surface. When a reference affects implementation
direction, scope, quality gates, review acceptance, or redistribution policy,
promote sanitized citation information according to
`docs/reference/bibliography/citation-governance.md`:

- external metadata belongs in `docs/reference/bibliography/references.md`
- source-family groupings belong in
  `docs/reference/bibliography/source-families.md`
- project claims and decisions belong in
  `docs/reference/bibliography/claim-map.md`

Do not promote raw cache notes, ignored files, copyrighted full text, long
excerpts, or score contents.

## Workflow

Before creating, updating, or validating cache files, read
[cache-schema.md](references/cache-schema.md). Choose the most stable available
identifier, preserve unknown values as empty or null, record the sources used
for verification, and update the small registry when adding a stable key.

Cache metadata may contain the dates and timestamps required by its freshness
schema. This is the machine-maintained cache exception to the repository's
durable-content date rule.
