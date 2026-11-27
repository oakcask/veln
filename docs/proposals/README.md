---
role: routing
update-when: A proposal is added, split, reclassified, completed, removed, its scope, implementation dependencies, or readiness changes, or proposal authoring-policy routes change.
---

# Proposals

The catalog in this directory contains only planned or incomplete work. Every
proposal page in this directory declares `role: proposal`. Proposal text is not
current language behavior unless the matching page under `../specification/`
also states it.

## Read First

- Current behavior: [Language Specification](../specification/README.md).

## Ready

- [Preserve Callback Results Through Tracing](tracing-result-polymorphism.md):
  generalize tracing wrappers after ordinary generic-function support; preserve
  callback values, typed errors, effects, and cleanup behavior.
- [Separate Cancellation Authority From Observation](cancellation-capabilities.md):
  use `CancelOwner` for cancellation and observation-only `CancelToken` for waits.
- [Return Cancellable Wait Outcomes Directly](cancellable-wait-outcomes.md):
  consolidate timer wait reporting and distinguish interruption from cancellation.
- [Preserve Progress in Network Write-all Results](network-write-progress.md):
  retain cumulative committed bytes and invalid-provider uncertainty on failure.
- [Multiple-handler Expressions](multiple-handler-syntax.md): add an ordered
  handler list and block body to `handle`, with outermost-first nesting,
  scoped context evaluation, and existing cleanup and provider behavior.
- [Generic Functions](generic-functions.md): implement explicit rank-one type
  schemes, independent call instantiation, scheme-preserving aliases,
  artifacts and tooling, and backend acceptance. Current declaration syntax
  and rigid definition checking are owned by the source-surface, type, and
  contract specifications; a separate prototype is not required for Ready.
- [Filesystem IO Feasibility](filesystem-io-feasibility.md): complete the
  remaining F4 provider and host positional IO conformance, uncertain
  host-seek cursor outcomes, host translation, open-mode, metadata,
  namespace-mutation, truncate, append, and sync. F5 covers cleanup.
  Direct-facade
  outcomes and provider-count validation, empty-request bypass, `read_exact`,
  `write_all`, bounded `io::copy`, scoped-fake open-resource seek,
  provider-side seek-beyond-end sequential writes and zero-gap policy,
  positional-cursor behavior, the bounded JVM rooted-host seek-beyond-end
  sequential extension and prevalidated invalid-seek preservation, rejected
  host write-publication release, F2, and F3 are implemented and belong to the
  current [filesystem and byte IO specification](../specification/standard-library-filesystem-io.md).
  The target does not require the production filesystem API to be implemented
  first.

## Design In Progress

- [Bodies for Conditional Branches and Arrow Clauses](control-flow-bodies.md):
  allow direct branch bodies and newline arrow bodies with implicit cleanup
  scopes, retain explicit `begin`, and permit unit conditionals without `else`;
  boundary and canonical-format decisions require review before Ready promotion.
- [Typed HTTP/2 Decision Projections](http2-typed-decisions.md): expose typed
  decision payloads instead of status-string/getter sequences; requires a
  complete view and retirement map for the affected decision families.
- [Single-receiver and Receiver-list Channel Waits](channel-wait-api.md):
  replace two-receiver selection with matching single-receiver and list wait
  families; result, selection-policy, public-type, and deterministic-wait
  feasibility decisions remain before Ready promotion.
- [Literal-expression Constant Folding](constant-folding.md): fold closed
  `Int` and `Bool` executable expressions while preserving diagnostics and
  runtime failures; requires full-range constant materialization and residual
  failure-span feasibility evidence before Ready promotion.
- [Standard-library Numeric Constants](numeric-constants.md): expose `Int`
  bounds and `Float` finite limits, positive minima, epsilon, infinities, and
  NaN through typed module values; requires scalar-data construction and
  non-finite comparison feasibility evidence before Ready promotion.
- [Floating-point Exponent Literals](floating-point-literals.md): add
  decimal and hexadecimal float exponent notation with explicit rounding and
  range errors; defer special-value tokens in favor of named constants. Independent of the
  constants API delivery.
- [Typed Predicate Logic and Flow Facts](typed-predicate-facts.md): define
  typed scalar predicates, value identity, flow transfer, and bounded proof
  outcomes as the shared prerequisite for callable interfaces and static
  contracts.
- [Contract-bearing Callable Types](contract-bearing-callable-types.md): retain
  bound pre/post interfaces through function values and public types; depends
  on typed predicate facts.
- [Static Contracts for Composable Calls](static-contracts.md): prove marked
  preconditions at calls and postconditions at returns, validate summaries,
  and enforce executable boundaries; depends on both language proposals above.

- Filesystem selection, host path conversion, shared byte IO, seek, provider
  ownership, and scoped fake filesystems; initial design decisions are fixed,
  but implementation selection depends on completing
  [Filesystem IO Feasibility](filesystem-io-feasibility.md).

## Planning Inventory

- The [agent-language-services umbrella](agent-language-services.md) remains a
  planning inventory only for the unimplemented transitive-dependency,
  remaining symbol, conformance-gate, and plugin work. Do not select the
  umbrella directly.

## Scope Decisions

- The one-virtual-thread-per-task JVM candidate did not satisfy its fixed
  controlled adoption gates with the retained wait algorithms. It is no longer
  active proposal work; see
  [JVM Lightweight Task Worker Selection](../reference/source-decisions/records/result-jvm-lightweight-task-worker-selection.md).
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

For proposals that add or change language syntax, also follow
[Proposal Canonical Form](../reference/documentation-authoring.md#proposal-canonical-form).

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
