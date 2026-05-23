---
name: docs-progressive-disclosure
description: Use when creating, reorganizing, splitting, merging, or maintaining documentation so agents and humans can start from short indexes and read only the task-relevant files.
---

# Docs Progressive Disclosure

## Goal

Keep documentation discoverable without forcing agents to read long historical or background files. Preserve durable context, but put short routing pages in front of long details.

## Workflow

1. Start from `docs/README.md` and identify the task the docs should support.
2. Measure the current shape with file lists, line counts, and headings before moving content.
3. Classify documents by purpose:
   - `reference/` for stable requirements, design, architecture, APIs, data formats, CI, and implementation policy.
   - `phases/` for phase plans, completion criteria, implementation notes, and open work.
   - `reviews/` for review findings, diagnostics evidence, quality gates, and rationale for plan changes.
4. Keep top-level and directory README files short. They should answer what to read, when to read it, and what to skip.
5. Move long details behind index pages instead of deleting or flattening them.
6. When a normally important file grows long, keep the original expected path as a short index and move the full body to a clearly named sibling such as `*-full.md` or `*-plan.md`.
7. Update relative links after moves and verify that Markdown links resolve.
8. If planned behavior, phase scope, diagnostics gates, or quality rationale change, keep the relevant docs aligned with the code change.

## Index Page Rules

- Index pages should be routing documents, not summaries of everything.
- Prefer sections like "Read First", "Read When", and "History" over long chronological lists.
- Put the newest or most actionable document first.
- Tell agents not to read old phases or old reviews unless they are doing history or rationale work.
- Link to detailed documents with relative paths.

## Split Criteria

Split or add an index when:

- A docs entry point is mostly a flat list of many files.
- A file mixes current guidance with historical review evidence.
- Agents would need to read more than one long file before knowing which file matters.
- A stable path is useful, but the content behind it has become too long for first-pass context.

Do not split when:

- The document is already short and has one clear audience.
- The split would create a directory with only one trivial file and no routing value.
- A local heading link is enough for the expected use.

## Verification

- Check file layout with `find docs -maxdepth 3 -type f | sort`.
- Check line counts with `wc -l` for changed index and detail files.
- Search for stale links with `rg`.
- Run a Markdown link existence check when files were moved.
