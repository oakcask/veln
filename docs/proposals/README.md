---
role: routing
update-when: A proposal is added, split, reclassified, completed, removed, or its scope, implementation dependencies, or readiness changes.
---

# Proposals

The catalog in this directory contains only planned or incomplete work. Every
proposal page in this directory declares `role: proposal`. Proposal text is not
current language behavior unless the matching page under `../specification/`
also states it.

## Read First

- Current behavior: [Language Specification](../specification/README.md).

## Ready

- Explicit runtime validation of a base-typed record against a format-neutral
  schema, composed with ordinary decoder functions:
  [schema-runtime-refinement-validation.md](schema-runtime-refinement-validation.md).
- With current typed module `let` source syntax and formatting as implemented
  prerequisites and same-module semantic analysis implemented, add imported
  lookup, visibility, execution, documentation publication, and
  language-service navigation:
  [module-immutable-values.md](module-immutable-values.md).
- Generic named-type variance and function assignability:
  [generic-and-function-variance.md](generic-and-function-variance.md).
- Explicit module import aliases and their lookup and casing rules:
  [explicit-import-aliases.md](explicit-import-aliases.md).
- Bounded buffered observation export and explicit shutdown:
  [observability.md](observability.md).

## Blocked

- Qualified immutable-value match refinement depends on module immutable value
  lookup, declaration identity, visibility, and execution semantics beyond the
  implemented parser and formatter support:
  [qualified-value-match-refinement.md](qualified-value-match-refinement.md).

## Design In Progress

- Lightweight task execution using JVM virtual threads; Java requirements,
  entry lifetime, scheduling guarantees, and resource budgets remain open:
  [lightweight-tasks.md](lightweight-tasks.md). Resolve its Open Questions
  before implementation selection.
- Portable filesystem paths, shared byte IO, seek, provider ownership, and
  scoped fake filesystems; source API and lifecycle questions remain open:
  [filesystem-io.md](filesystem-io.md). Resolve its Open Questions before
  implementation selection.

## Planning Inventory

- The [agent-language-services umbrella](agent-language-services.md) remains a
  planning inventory only for the unimplemented transitive-dependency,
  remaining symbol, conformance-gate, and plugin work. Do not select the
  umbrella directly.

## Scope Decisions

- The hypothetical missing-provider descriptor diagnostic has no independently
  justified production provider feature. It is no longer active proposal work;
  see [Variant Descriptor Provider Boundary](../reference/source-decisions/variant-descriptor-provider.md).

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
