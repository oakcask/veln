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
9. Add or update the `role:` and `review-when:` fields in the YAML frontmatter
   of every Markdown document that the change adds or modifies under `docs/`.
   Add `authority:` and `status:` only when the metadata rules require them.

## Document Metadata

Every added or modified Markdown document under `docs/` must declare one role:

- `routing`: an index or selection route that does not define behavior.
- `specification`: current behavior or a current project contract.
- `proposal`: planned or incomplete behavior.
- `reference`: stable requirements, policy, rationale, or source support.
- `review`: bounded findings, diagnostics evidence, or quality-gate results.
- `implementation-record`: completed proposal history or completion evidence.

Use `authority:` only when the document itself can support a claim:

- A `specification` must use `authority: normative`.
- A `reference` must use `authority: normative` or `authority: supporting`.
- A `review` or `implementation-record` may use `authority: supporting`.
- A `routing` or `proposal` must not declare `authority:`.

Use `status:` only for an exceptional lifecycle state. The allowed states are
`closed`, `rejected`, and `superseded`. A `specification` cannot declare a
status. A `routing` document can be `closed` or `superseded`. A `proposal` can
be `closed`, `rejected`, or `superseded` after it is moved out of
`docs/proposals/`. A `reference`, `review`, or `implementation-record` can be
`superseded`. Do not add `status:` for an active proposal, ordinary route, or
current supporting record. Do not put a `Status:` label in the document body.

```markdown
---
role: specification
authority: normative
review-when: The documented behavior or its executable evidence changes.
---
```

```markdown
---
role: routing
review-when: A routed document is added, moved, or reclassified.
---
```

## Review Triggers

Put YAML frontmatter at the start of the document. Add one single-line
`review-when:` field. Name an observable project-state change that would make a
maintainer recheck the document. Quote the value when YAML punctuation could
make it ambiguous. Do not use calendar schedules or vague values such as
`periodically`, `regularly`, `as needed`, `when necessary`, `always`, or `TBD`.

```markdown
---
review-when: The documented command output or its executable evidence changes.
---

# Command Output
```

Choose the trigger from the document's purpose:

- Specification: review when the documented behavior, public contract, or
  authoritative executable evidence changes.
- Proposal: review when its acceptance evidence, scope, dependencies, or
  implementation status changes.
- Reference or decision record: review when its authority, replacement,
  supporting evidence, or the decision boundary changes.
- Routing page: review when a routed document is added, moved, reclassified,
  or no longer answers the routed task.
- Historical record: review when the record is superseded, its links or
  evidence become invalid, or current documentation starts relying on it as an
  authority.

Use the narrowest sufficient trigger. State multiple related conditions in the
same field when any one of them requires review. Do not add a second
`review-when:` field.

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
- Use `docs/proposals/` only for `role: proposal` targets that are not fully
  implemented. The directory README uses `role: routing`. Remove or relocate
  rejected, superseded, implemented, and otherwise closed proposal pages.
- Keep implementation gaps, verification evidence, and correction lists in the
  matching proposal or reference page.
- When prose and executable evidence disagree, update the implementation,
  executable evidence, or prose together.

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
- Run `node workflow-scripts/check-doc-frontmatter.mjs` with the changed
  Markdown paths. CI applies the same check to added, modified, and moved
  documents.
