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

- [Named ADT Variant Fields](named-variant-fields.md): distinguish named and
  positional variants, add named construction and patterns, preserve evaluation
  order through formatting and tooling, and complete stdlib adoption plus its
  API design-guideline update before closing the proposal.
- [Preserve Callback Results Through Tracing](tracing-result-polymorphism.md):
  generalize tracing wrappers after ordinary generic-function support; preserve
  callback values, typed errors, effects, and cleanup behavior.
- [Separate Cancellation Authority From Observation](cancellation-capabilities.md):
  use `CancelOwner` for cancellation and observation-only `CancelToken` for waits.
- [Multiple-handler Migration and Acceptance](multiple-handler-syntax.md):
  expand migration to other multiline-body and comment-preserving cases, and
  complete the remaining task, provider-capture, and direct-IO acceptance
  cases.
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
- [Consistent Expression and Block Forms](control-flow-bodies.md): implement
  direct conditional bodies, unit conditionals without `else`, and paired
  `defer` forms with the specified token boundaries, cleanup timing, and
  canonical formatting; parser/backend/formatter checks establish completion.
- [Standard-library Numeric Constants](numeric-constants.md): implement typed
  module values for `Int` bounds and `Float` limits and special values through
  the specified narrow scalar-data boundary; construction, forwarding, API,
  and discovery checks are completion requirements.
- [Floating-point Exponent Literals](floating-point-literals.md): implement
  decimal and hexadecimal exponent notation with the specified token,
  conversion-extension, and tooling rules on the implemented shared
  dotted-decimal conversion boundary. Independent of numeric constants API
  delivery.
- [Filesystem And Shared Byte IO](filesystem-io.md): implement the remaining
  production filesystem, shared IO, and adapter rollout under the fixed
  capability, outcome, and ownership contract. Complete and reuse F4–F5
  conformance before production delivery; it is not a Ready-promotion gate.
- [Predicate Work-budget Feasibility](predicate-work-budget-feasibility.md):
  investigate the mandatory comparison fragment and deterministic accounting
  policy using an isolated model and bounded workloads, independently of the
  production verifier and static-contract implementation.

## Design In Progress

- [Typed HTTP/2 Decision Projections](http2-typed-decisions.md): complete the
  per-family public view, payload, signature, and status/getter retirement map.
  No prototype or generic-function implementation is required for Ready.
- [Single-receiver and Receiver-list Channel Waits](channel-wait-api.md):
  resolve owning declarations, type/constructor paths, imports, and tooling
  publication of channel error and policy types alongside compiler-known calls.
  Deterministic waits, rendezvous preservation, and migration audits are
  implementation acceptance.
- [Typed Predicate Logic and Flow Facts](typed-predicate-facts.md): resolve the
  mandatory-case boundary and deterministic work policy using the separately
  Ready budget investigation; full sema/consumer acceptance is delivery work.
- [Contract-bearing Callable Types](contract-bearing-callable-types.md):
  settle clause ordering, nesting, effect-row placement, canonical source,
  and the predicate prerequisite's unresolved work policy. Grammar prototypes
  and passing type/consumer tests are not Ready prerequisites.
- [Static Contracts for Composable Calls](static-contracts.md): resolve the
  predicate and callable contracts, source-dependency/recursive-summary
  invalidation transitions, and concrete diagnostic/documentation schemas.
  Full body, lowering, and publication verification is implementation acceptance.

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

For acceptance models, Ready promotion, bounded feasibility work, canonical
source, and related work, follow
[Proposal Authoring Policy](../reference/proposal-authoring.md).

## Update When

Promote observable behavior to executable evidence under
`../../examples/specification/` first when practical, then update the smallest
matching specification page. Delete completed, rejected, superseded, and
otherwise closed proposals and remove them from this catalog. Preserve only
durable rationale that remains useful independently of proposal completion
under `../reference/source-decisions/`.
