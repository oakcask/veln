# Discussion Result: Satisfy Unknown Severity

## Picked Question

- What severity should unknown but well-formed `satisfy` predicates have
  during ordinary `check`, and which status should block any future repair
  application?

## Decision

During ordinary `veln check`, a well-formed `satisfy` predicate that cannot be
statically discharged should remain `severity: "hint"` while the associated
hole is unfilled.

The hole diagnostic should keep the program-level check result `partial`, not
`failed`, unless another parse, type, contract, or effect diagnostic is an
error. The `satisfy` constraint should be represented as a hard repair
constraint with `validation_status: "valid_unknown"` and
`repair_status: "blocked_until_discharged"` or an equivalent first-slice enum.

A future automatic repair-application command must not silently apply a
candidate when any hard `satisfy` predicate remains `valid_unknown`,
`failed_static`, or `invalid`. It may apply only when every hard predicate is
`discharged`, or when a later explicit user-confirmed mode records that unknown
constraints were accepted outside the automatic path.

## Rationale

Typed-hole systems are designed to keep incomplete programs analyzable. Hazel's
work on live functional programming with typed holes supports treating the
unfilled hole and its local context as useful partial-program information
rather than as a normal compile failure. That matches Veln's existing
`hole.unfilled` decision: an ordinary check should give agents repair guidance
without conflating "not filled yet" with "the source is wrong".

At the same time, an unknown `satisfy` predicate is not evidence that a repair
is correct. Liquid Types demonstrates the value of a deliberately restricted,
solver-friendly refinement language: the tool can claim success only for
obligations it can validate within that controlled fragment. SMT-LIB makes the
same operational distinction for satisfiability checks: `unknown` is an
inconclusive outcome, not a proof.

LSP and SARIF both separate diagnostic severity from tool-specific metadata.
That split is useful here. The visible severity can stay a low-noise hint in
the normal edit/check loop, while the structured repair metadata records a
hard gate for automatic edits. Agents get a stable rule: unknown constraints
are visible and actionable, but they do not authorize code insertion.

This also preserves the boundary between `satisfy` and contracts. A `satisfy`
predicate is repair guidance attached to a hole, not an implicit runtime
contract. If users want a condition to be enforced after the hole is filled,
they should write an explicit `require`, `ensure`, assertion, or test.

## First-Slice Rule

- An unfilled hole with a well-formed but undischarged `satisfy` predicate
  reports `severity: "hint"` during ordinary `veln check`.
- The check result remains `partial` when the only remaining issues are
  unfilled holes or unknown hole constraints.
- A `satisfy` constraint record should include a machine-readable status such
  as `validation_status: "valid_unknown"` and
  `repair_status: "blocked_until_discharged"`.
- `valid_unknown` means the predicate is syntactically valid, type-correct,
  pure under the contract-predicate rules, and not statically proved or
  disproved for the relevant candidate context.
- `failed_static` means the checker can disprove the predicate for a proposed
  candidate; this blocks repair and should be reported as an error in repair
  validation output.
- `invalid` means the predicate is not a legal `satisfy` predicate; this blocks
  repair and should use the existing `hole.satisfy_invalid` diagnostic family.
- Automatic repair application requires all hard `satisfy` predicates for the
  candidate to be `discharged`.
- Tests, examples, or ordinary execution may be reported as supporting
  evidence, but they do not change `valid_unknown` into `discharged` in the
  first slice.
- A future user-confirmed repair mode may accept unknown constraints, but it
  must record that acceptance explicitly and must not be the default automatic
  behavior.

## Diagnostic Shape Addition

The first implementation can extend each hole `constraints` item like this:

```json
{
  "kind": "satisfy",
  "text": "candidate.port > 0",
  "candidate_binding": "candidate",
  "validation_status": "valid_unknown",
  "repair_status": "blocked_until_discharged"
}
```

The exact enum names may still change with the prototype `details` payload, but
the semantic split should remain stable: ordinary check severity is low, and
automatic repair authorization is strict.

## Open Detail

The future repair command still needs a full result schema for candidate
preview, acceptance, rejection, and user-confirmed override. This decision only
settles the ordinary check severity and the default automatic-application
gate.

The source syntax for attaching `satisfy` predicates remains resolved
elsewhere. This result assumes the predicate and candidate binding already
exist in the surface AST or recovered hole metadata.

## References

- Omar, C., Voysey, I., Chugh, R., & Hammer, M. A. (2019). Live functional
  programming with typed holes. *Proceedings of the ACM on Programming
  Languages*, 3(POPL), 1-32. https://doi.org/10.1145/3290327
- Rondon, P. M., Kawaguchi, M., & Jhala, R. (2008). Liquid types.
  *PLDI 2008*, 159-169. https://doi.org/10.1145/1375581.1375602
- Barrett, C., Fontaine, P., & Tinelli, C. (2025). *The SMT-LIB Standard:
  Version 2.7*. SMT-LIB.
  https://smt-lib.org/papers/smt-lib-reference-v2.7-r2025-07-07.pdf
- Microsoft. (2026). *Language Server Protocol Specification - 3.17:
  Diagnostic*.
  https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#diagnostic
- Fanning, M. C., & Golding, L. J. (Eds.). (2020). *Static Analysis Results
  Interchange Format (SARIF) Version 2.1.0*. OASIS Standard.
  https://docs.oasis-open.org/sarif/sarif/v2.1.0/os/sarif-v2.1.0-os.html

## Consequence

The normal edit/check loop stays quiet and useful for incomplete programs, but
automatic repair remains conservative. Agents can rank and explain candidate
fills with unknown constraints, yet they cannot treat those candidates as
approved edits until the hard `satisfy` predicates are discharged or explicitly
accepted by a later user-confirmed workflow.
