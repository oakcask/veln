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

- Complete bounded inference beyond implemented source-ADT and compiler-owned
  covariant and structural-record joins, including contravariant meets,
  structural function joins, and contributed-candidate fallback; add
  dependency variance-metadata invalidation, stable formatter, package, LSP,
  and MCP presentation, nested incompatibility diagnostics, and whole-analysis
  and language-service failure preservation:
  [generic-and-function-variance.md](generic-and-function-variance.md).

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
Ready. If Ready contains no suitable implementation target, report that there
is no target instead of selecting work from another section.

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
