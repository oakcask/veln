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
   - `proposals/` for planned or incomplete behavior.
   - `reviews/` for review findings, diagnostics evidence, quality gates, and rationale for plan changes.
4. Keep top-level and directory README files short. They should answer what to read, when to read it, and what to skip.
5. Move long details behind index pages instead of deleting or flattening them.
6. When a normally important file grows long, keep the original expected path as a short index and move the full body to a clearly named sibling such as `*-full.md` or `*-plan.md`.
7. Update relative links after moves and verify that Markdown links resolve.
8. If planned behavior, phase scope, diagnostics gates, or quality rationale change, keep the relevant docs aligned with the code change.

## Placement Rules

- `docs/specification/` is the first stop for implemented language behavior.
- Keep prose specification pages thin: they should route, summarize, and
  explain executable or mechanically checked evidence when practical.
- Prefer `examples/specification/`, generated grammar, compiler tests, checked
  fixtures, or CLI harness cases for behavior that can be expressed
  mechanically.
- Use `docs/reference/source-decisions/` for implemented rationale and decision
  history.
- Use `docs/reference/implemented-proposals/` for completed proposal history
  and completion evidence, not current behavior.
- Use `docs/proposals/` for proposed targets that are not fully implemented.
- Keep implementation gaps, verification evidence, and correction lists in the
  matching proposal or reference page.
- When prose and executable evidence disagree, update the implementation,
  executable evidence, or prose together.

## Status Labels

Use durable document labels narrowly:

- `implemented`: current behavior or implemented rationale supported by code
  and tests.
- `proposed`: committed proposal text whose implementation is absent or
  incomplete.
- `routing`: an index or selection route that does not define behavior.
- `closed`: a former proposal route preserved only for old links.
- `superseded`: another document replaces this one.
- `rejected`: the project decided not to pursue the design.

`Status:` describes document authority and placement, not whether every idea in
the file exists in the product. Proposal pages should not use
`Implementation:` to describe current behavior.

## Index Page Rules

- Index pages should be routing documents, not summaries of everything.
- Prefer sections like "Read First", "Read When", and "History" over long chronological lists.
- Put the newest or most actionable document first.
- Tell agents not to read old reviews unless they are doing history or rationale work.
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
