---
role: routing
review-when: A proposal is added, moved, reclassified, completed, or removed.
---

# Proposals

The catalog in this directory contains only planned or incomplete work. Every
proposal page in this directory declares `role: proposal`. Proposal text is not
current language behavior unless the matching page under `../specification/`
also states it.

## Read First

- Current behavior: [Language Specification](../specification/README.md).
- Completed proposal history:
  [Implemented Proposal Records](../reference/implemented-proposals/README.md).

## Catalog

### Packages and Source Discovery

- [Manifest Package Boundaries](manifest-package-boundaries.md): make a nested
  `veln.toml` an ownership boundary for command, editor, dependency, and
  checksum source discovery, and stop treating `target` as a reserved source
  directory name.

## Selection Rule

Before implementing a proposal slice, compare it with the matching
specification page and executable cases. Do not select work that is already
covered there or only extends a numbered, width-based, arity-based, route-count,
or diagnostic-id sequence. Such work needs a concrete new capability and a
bounded stopping condition.

## Proposal Shape

Express observable targets as structured acceptance cases, decision tables,
state-transition tables, executable models, or another directly verifiable
form when practical. Map those targets to the tests, fixtures, doctests,
benchmarks, or executable specifications that will verify implementation.
Keep prose for scope, rationale, non-goals, and constraints that cannot
reasonably be expressed in the primary verification medium. Do not describe
planned evidence as already passing.

State proposed behavior declaratively as an externally observable contract.
Keep internal algorithms and ordered procedures outside normative behavior
unless they are required design constraints. Use Simplified Technical English
style when those details need prose explanation.

## Update When

Promote observable behavior to executable evidence under
`../../examples/specification/` first when practical, then update the smallest
matching specification page. Move completed proposal history to
`../reference/implemented-proposals/` and remove it from this catalog.
Remove rejected, superseded, and otherwise closed proposals from this
directory. Preserve durable rationale under `../reference/` when it remains
useful.
