---
name: verifiable-specification-writing
description: Use when creating, substantially revising, or reviewing documentation that specifies behavior, requirements, acceptance criteria, protocols, stateful processes, performance properties, grammar, schemas, APIs, commands, or observable outputs. Applies to proposals, specifications, design notes, reference material, and other normative documentation regardless of directory.
---

# Verifiable Specification Writing

## Goal

Make normative claims precise, reviewable, and resistant to drift by putting
behavior in the strongest practical verification medium. Keep natural-language
text subordinate to primary evidence when the claim can be expressed
mechanically or structurally.

Specify behavior declaratively as an externally observable contract. State
inputs, outcomes, failures, visible state transitions, and invariants without
prescribing internal control flow, data structures, or operation order unless
those details are required constraints.

Apply this rule by content, not directory. A reference or design page that
defines behavior is in scope. Pure routing, rationale, history, bibliography,
and explanation are not normative unless they also make behavioral claims.

## Workflow

1. Identify each normative claim and the external observation that would prove
   or disprove it. Separate internal design explanation, rationale, scope, and
   non-goals from those claims.
2. Choose the strongest practical primary medium using the selection guide
   below. Prefer an existing repository harness or artifact format.
3. Write or update the primary artifact before expanding prose when practical.
4. Cover success, boundary, failure, and state-preservation behavior that is
   material to the claim. Do not rely on one happy-path example to specify a
   general rule.
5. Route nearby prose to the primary artifact. Summarize its meaning without
   duplicating all of its cases as a second source of truth.
6. Record how the artifact is checked locally or by CI. If it is not checked,
   label its authority accurately and state what will verify it.
7. If only prose is practical, state the reason briefly and make the prose
   falsifiable with explicit inputs, outcomes, boundaries, or invariants.

## Declarative Boundary

- Describe what users, callers, tools, or interoperating systems can observe.
- State inputs, outcomes, failures, invariants, and externally visible state
  transitions.
- Do not turn the current implementation algorithm into a normative
  requirement.
- Include an internal algorithm, data structure, or operation order only when
  it is needed to explain the design or when compatibility, safety,
  performance, or another explicit constraint makes it significant.
- Mark internal explanations as rationale or implementation guidance unless
  they are intentionally normative.
- Explain why an internal detail is normative when the specification must
  constrain it.

## Medium Selection

- API examples and documented source behavior: use executable doctests when
  the harness supports the behavior.
- Input, output, diagnostics, commands, or serialization: use table-driven
  tests, checked fixtures, CLI cases, or executable examples.
- Grammar, schemas, and source surfaces: use executable grammar, schemas, and
  accepted and rejected fixtures.
- Stateful or protocol behavior: use a transition table with current state,
  event, guard, next state, outputs, and failures. Map material rows to tests.
- Rule combinations: use decision tables or truth tables and cover the rows
  mechanically where practical.
- Performance claims: use benchmarks with a named workload, metric, comparison
  method, and noise policy. Do not use a benchmark to specify functional
  behavior or claim a stable threshold on an uncontrolled runner.
- Multi-step overviews: use a flowchart as a supporting view. Prefer deriving
  it from the same model as tests; otherwise name the transition table, tests,
  or executable model as authoritative.
- Rationale, scope, non-goals, provenance, and genuinely non-mechanical
  constraints: use concise prose.

## Simplified Technical English Style

When an internal algorithm or ordered procedure needs prose explanation, use
Simplified Technical English style. Do not claim formal conformance to a
controlled-language standard.

- Use one action or condition per sentence.
- Use short sentences with an explicit subject and verb.
- Use the same term for the same concept. Do not introduce stylistic synonyms.
- Define abbreviations and specialized terms before using them.
- Prefer active voice when the actor is relevant.
- Put a condition before the action that depends on it.
- Use numbered steps when order is significant.
- Use one imperative action per procedural step.
- State the expected result when it is not obvious.
- Avoid ambiguous pronouns, nested conditions, idioms, and informal metaphors.

## Planned and Current Behavior

For planned behavior, provide a structured acceptance model and map it to the
tests, fixtures, doctests, benchmarks, or executable specification that will
verify implementation. Do not imply that planned evidence is already running
or passing.

For current behavior, add or update checked evidence when practical. Keep
planned examples out of current specification routes until implementation and
verification agree.

## Authority and Drift

- Name the authoritative artifact when prose, tables, diagrams, generated
  pages, and tests describe the same behavior.
- Prefer generating secondary views from the authoritative artifact.
- Update all affected representations together when generation is impractical.
- Treat disagreement as a defect; do not resolve it by silently declaring the
  least verifiable representation authoritative.
- Do not add a diagram merely to satisfy this guardrail. Use it only when it
  makes control flow or state relationships materially clearer.

## Review Checklist

- Every normative claim has an observable acceptance condition.
- The chosen medium is more directly verifiable than practical alternatives.
- State tables include failures and unchanged-state outcomes where relevant.
- Benchmarks identify workload, metric, comparison, and noise handling.
- Prose routes to or explains primary evidence instead of duplicating it.
- Normative text describes externally observable behavior rather than an
  accidental implementation strategy.
- Any normative internal constraint explains why the detail must be
  constrained.
- Internal algorithm and procedure sections are distinguishable from
  behavioral requirements.
- Algorithm and procedure prose follows Simplified Technical English style.
- Planned evidence is not presented as implemented or passing.
- Verification commands or CI routes are discoverable.
- Any prose-only exception explains why stronger representation is not
  practical.
