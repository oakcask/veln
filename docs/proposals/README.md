---
role: routing
update-when: A proposal is added, moved, reclassified, completed, removed, or its scope or readiness changes.
---

# Proposals

The catalog in this directory contains only planned or incomplete work. Every
proposal page in this directory declares `role: proposal`. Proposal text is not
current language behavior unless the matching page under `../specification/`
also states it.

## Read First

- Current behavior: [Language Specification](../specification/README.md).

## Ready

- Standard-library structured logs, events, metrics, traces, explicit context
  propagation, and substitutable exporters, building on canonical and retained
  call-site locations:
  [observability.md](observability.md).
- Remaining standard-library TCP system handler, typed host failures,
  resource lifecycle, and duplex transport adapter, building on the
  implemented public network effect, direct facade, and `write_all` helper:
  [standard-library-networking.md](standard-library-networking.md).
- Remaining ADT variant-refinement support for type-alias presentation and
  visibility, qualified immutable values after a separate source-surface
  prerequisite, schema boundaries,
  package documentation, command-wide enforcement, LSP, and remaining MCP
  diagnostics, navigation, rename, package-signature, and saved-state behavior:
  [adt-variant-refinement-types.md](adt-variant-refinement-types.md).

## Blocked

- Generic named-type and function variance remains separate from ADT variant
  refinement and is blocked until its constructor classifications and complete
  callable composition table are decided:
  [generic-and-function-variance.md](generic-and-function-variance.md).
- Explicit import-alias casing is blocked until an owning proposal defines the
  syntax and lookup contract:
  [identifier-casing-explicit-import-aliases.md](identifier-casing-explicit-import-aliases.md).
- The [agent-language-services umbrella](agent-language-services.md) remains a
  planning inventory only for the unimplemented transitive-dependency,
  remaining symbol, conformance-gate, and plugin work. Do not select the
  umbrella directly.

## Selection Rule

Before implementing a proposal, compare it with the matching specification
page and executable cases. Select only a complete proposal page listed under
Ready. If Ready contains no suitable implementation target, report
that there is no target instead of selecting work from Blocked.

Do not select work that is already covered by the current specification or
only extends a numbered, width-based, arity-based, route-count, or diagnostic-id
sequence. Such work needs a concrete new capability.

## Proposal Shape

For authoring and review, follow
[Proposal Related Work](../reference/documentation-authoring.md#proposal-related-work)
to connect source evidence with design choices and alternatives.

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
matching specification page. Delete completed, rejected, superseded, and
otherwise closed proposals and remove them from this catalog. Preserve only
durable rationale that remains useful independently of proposal completion
under `../reference/source-decisions/`.
