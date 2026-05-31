---
name: proposal-implementation-audit
description: Use when implementing, completing, reviewing, or cleaning up work that starts from docs/proposals or turns planned proposal text into implemented Veln behavior. Ensures implemented behavior is promoted to docs/specification and examples/specification, and completed proposal records are moved out of docs/proposals.
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

1. Identify the proposal page or proposal section that the code change
   implements.
2. Compare the implemented behavior with the matching short page under
   `docs/specification/`, starting from `docs/specification/topic-map.md` when
   the target page is unclear.
3. Add or update primary executable specification evidence first when the
   behavior can be checked mechanically. Use `examples/specification/` for
   observable source, diagnostics, command output, JSON, formatting, generated
   docs, runtime output, tests, or repair output. Use
   `docs/specification/source-surface-executable.pl` or nearby checked
   fixtures for source-surface grammar behavior.
4. Update the smallest matching `docs/specification/` prose page only after the
   executable evidence is in place or after deciding that no practical
   executable evidence exists. Keep prose thin as a route, summary, or derived
   explanation of the executable specification.
5. Audit `docs/proposals/README.md` and the implemented proposal page:
   completed behavior must not remain cataloged as planned or future work.
6. For fully completed proposals, move the historical record to
   `docs/reference/implemented-proposals/` and update that directory's
   `README.md`. Remove it from the proposals catalog.
7. For partially completed proposals, keep only the unimplemented remainder in
   `docs/proposals/`; rewrite the page and catalog entry so they clearly name
   the remaining planned work.
8. Before finishing, search for stale proposal-only wording around the changed
   feature and make sure current-behavior claims point to `docs/specification/`
   or checked examples instead of proposal text.

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
- Any proposal text left behind describes only unimplemented follow-up work.
- The final response names the specification and example updates, or explicitly
  states why none were needed.

## Placement Rules

- Do not cite `docs/proposals/` as current behavior after implementation.
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
