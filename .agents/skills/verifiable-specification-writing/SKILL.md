---
name: verifiable-specification-writing
description: Use when creating, substantially revising, or reviewing documentation that specifies behavior, requirements, acceptance criteria, protocols, stateful processes, performance properties, grammar, schemas, APIs, commands, or observable outputs. Applies to proposals, specifications, design notes, reference material, and other normative documentation regardless of directory.
---

# Verifiable Specification Writing

## Goal

Turn normative claims into precise, reviewable contracts backed by the strongest
practical evidence.

## Authority

Read
[documentation-authoring.md](../../../docs/reference/documentation-authoring.md),
especially `Behavior Specifications`, before writing or reviewing normative
content. That document owns the specification-writing rules. This skill defines
the procedure for applying them.

## Workflow

1. Identify the user, product, interoperability, safety, or maintenance outcome
   that justifies each proposed requirement.
2. Separate normative claims from rationale, scope, non-goals, historical
   evidence, and implementation guidance.
3. State the external observation that would prove or disprove each normative
   claim.
4. Identify an authority independent of the implementation being checked.
5. Select the strongest practical verification medium using the authoring
   policy and an existing repository harness or artifact format.
6. Write or update the primary artifact before expanding prose when practical.
7. Cover material success, boundary, failure, and state-preservation outcomes.
8. Route nearby prose to the primary artifact without duplicating its cases as
   another source of truth.
9. Record how the artifact is checked locally or by CI. If it is not checked,
   label its authority accurately and state what will verify it.
10. If only prose is practical, state why and make the claim falsifiable with
    explicit inputs, outcomes, boundaries, or invariants.

## Review Procedure

- Compare every normative claim with the acceptance condition and authority
  identified for it.
- Check that planned evidence is not described as implemented or passing.
- Check that prose describes observable behavior rather than an incidental
  implementation strategy.
- Check that stateful rules cover failures and unchanged-state outcomes.
- Check that performance claims name their workload, metric, comparison, and
  noise handling.
- Check that repeated values are not presented as independent corroboration
  when they derive from the same source.
- Check that prose, executable evidence, generated views, and implementation do
  not disagree.
- Confirm that verification commands or CI routes are discoverable.
