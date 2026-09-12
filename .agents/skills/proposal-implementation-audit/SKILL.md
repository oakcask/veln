---
name: proposal-implementation-audit
description: Select, implement, review, or close proposal work while keeping current specifications and proposal state aligned.
---

# Proposal Implementation Audit

## Goal

Prevent implemented behavior from remaining documented only as proposal text.
When proposal work lands, the current specification and executable examples
must become the source of truth, and proposal records must no longer describe
completed behavior as future work.
Executable specification evidence is primary. Natural-language pages under
`docs/specification/` are supporting routes, summaries, or derived prose that
must stay aligned with executable examples, checked fixtures, and executable
grammar.

## Workflow

1. When selecting work, choose only from the Ready section of
   `docs/proposals/README.md`. If it has no ready target, report that there is no
   target rather than selecting from another section.
2. Identify the proposal page or proposal section that the code change
   implements.
3. Treat proposal indexes, routing pages, and broad follow-up inventories as
   navigation, not implementation targets. Stop when the target is implemented,
   closed, superseded, rejected, or already covered by `docs/specification/`.
4. Compare the implemented behavior with the matching short page under
   `docs/specification/`, starting from `docs/specification/topic-map.md` when
   the target page is unclear.
5. Keep the comparison scoped to the chosen proposal page. Do not use nearby
   design-wall, broad follow-up, or implemented-history text as requirements
   unless the selected proposal page points to it.
6. Add or update primary executable specification evidence first when the
   behavior can be checked mechanically. Use `examples/specification/` for
   observable source, diagnostics, command output, JSON, formatting, generated
   docs, runtime output, tests, or repair output. Use
   `docs/specification/source-surface-executable.pl` or nearby checked
   fixtures for source-surface grammar behavior.
7. Update the smallest matching `docs/specification/` prose page only after the
   executable evidence is in place or after deciding that no practical
   executable evidence exists. Keep prose thin as a route, summary, or derived
   explanation of the executable specification.
8. Audit `docs/proposals/README.md` and the implemented proposal page:
   completed behavior must not remain cataloged as planned or future work.
9. For fully completed proposals, move the historical record to
   `docs/reference/implemented-proposals/` and update that directory's
   `README.md`. Remove it from the proposals catalog.
10. For rejected, superseded, or otherwise closed proposals, remove the page
    from `docs/proposals/` and its catalog. Preserve useful rationale in the
    matching `docs/reference/` area instead of keeping a closed proposal route.
11. For partially completed proposals, keep only the unimplemented remainder in
    `docs/proposals/`; rewrite the page and catalog entry so they clearly name
    the remaining planned work.
12. Before finishing, search for stale proposal-only wording around the changed
    feature and make sure current-behavior claims point to `docs/specification/`
    or checked examples instead of proposal text.

## Specification Update Routes

When the matching current-behavior page is unclear, read
[specification-routes.md](references/specification-routes.md). Do not read it
when the selected proposal already points to the relevant specification and
executable evidence.

## Review Checklist

- The behavior implemented by code is present in executable specification
  evidence when practical, such as `examples/specification/`,
  `docs/specification/source-surface-executable.pl`, checked fixtures,
  compiler tests, or CLI harness cases.
- Natural-language pages under `docs/specification/` summarize, route to, or
  explain the executable evidence instead of being the only source of truth
  when mechanical coverage is practical.
- `docs/proposals/README.md` no longer lists completed work as planned.
- Completed proposal records live under
  `docs/reference/implemented-proposals/`, not under `docs/proposals/`.
- Rejected, superseded, and otherwise closed proposal pages do not remain under
  `docs/proposals/`.
- Any proposal text left behind describes only unimplemented follow-up work.
- The final response names the specification and example updates, or explicitly
  states why none were needed.

## Placement Rules

- Do not cite `docs/proposals/` as current behavior after implementation.
- Do not move text from a proposal into `docs/specification/` until current
  code and tests support it.
- Do not move rationale-only material into `docs/specification/`; keep
  rationale in `docs/reference/` and link only when the specification needs a
  route to context.
- Do not preserve obsolete future-tense proposal wording for behavior that is
  now implemented.
- Do not add broad prose when a focused executable fixture or checked example
  can carry the behavior.
- Do not treat natural-language specification prose as the primary artifact
  when the behavior can reasonably be expressed as executable specification
  evidence.
