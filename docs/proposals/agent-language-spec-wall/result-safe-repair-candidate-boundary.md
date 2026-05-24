# Discussion Result: Safe Repair Candidate Boundary

Status: accepted-proposal

## Picked Question

- What should `safe repair` mean before Veln has a dedicated `repair` command,
  and what evidence should a future automatic repair workflow need before it
  applies a candidate edit?

## Decision

`safe repair` should initially mean an auditable, machine-readable repair
candidate, not an automatically applied edit and not a correctness guarantee.

The first implementation should keep `veln repair` deferred. Existing commands
may still expose repair-relevant facts: hole candidate queries, structured
diagnostic evidence, related spans, and verification hints. If a future
prototype emits concrete candidate edits, each candidate must be explicitly
unapplied and must carry its target, reason, evidence, known limits, blocking
obligations, and recommended verification command.

Automatic application requires a stricter later gate. A candidate can be
automatically applied only when all hard static obligations that Veln knows how
to check have passed, every hard hole `satisfy` constraint is discharged, and
the candidate has been verified by the configured test or example command. Even
then, the result should be described as "validated for these obligations", not
as universally safe.

Passing tests, examples, or doctests are supporting evidence. They are not
enough by themselves to convert a candidate into a correctness claim.

## Rationale

Automatic repair research supports the usefulness of generated patch
candidates, but it also gives a direct warning about overclaiming safety.
GenProg shows that existing test suites can serve as a practical repair oracle
for generating variants of real programs. Monperrus's survey frames automatic
repair more broadly around different oracles, including tests, contracts,
models, and crashing inputs. For Veln, that supports carrying oracle evidence
as structured data rather than treating repair as pure prose.

Qi, Long, Achour, and Rinard draw the important boundary for this decision:
a patch that passes the available tests is plausible, but plausibility is not
semantic correctness. That is especially relevant to an agent-oriented
language, because agents will be tempted to treat a green command as permission
to edit. Veln should make the evidence visible while keeping the authorization
rule conservative.

Prophet is useful precedent for ranking candidates with learned correctness
signals before validation. It supports exposing reasons, ranking features, and
validation outcomes separately. It does not remove the need for explicit gates,
because ranking helps choose what to try; it does not prove that an edit is
safe for all intended behavior.

This also aligns with the existing diagnostic shape decisions. LSP and SARIF
separate the diagnostic envelope from tool-specific metadata, and Veln already
uses prototype `details` payloads for repair-routing facts. Candidate repair
records should follow the same pattern: small stable routing fields first,
evolving evidence payloads behind the prototype boundary.

## First-Slice Rules

- `veln repair` remains out of scope for the first implementation.
- Ordinary `veln check --json` diagnostics may include repair-relevant context,
  but first-slice diagnostics must not imply that an edit has been authorized.
- A concrete candidate edit, if emitted by a prototype, must include:
  `candidate_id`, source-relative target span or `node_id`, edit summary,
  reason, evidence list, known limits, blocking obligations, verification hint,
  and `application_policy: "manual_review_required"`.
- Candidate evidence should distinguish at least static facts, contract facts,
  hole constraints, effect facts, tests/examples, and ranking signals.
- Tests and examples may appear as supporting evidence only. They do not
  discharge contract or `satisfy` obligations unless those obligations are
  checked by the corresponding Veln mechanism.
- A future automatic-application path must fail closed when any hard obligation
  is `unknown`, `failed_static`, invalid, unverified, or outside the candidate's
  evidence scope.
- A future user-confirmed override may accept unknown obligations, but it must
  record the override explicitly and must not be the default automatic path.

## Candidate Sketch

The exact schema remains prototype-level, but the semantic split should look
like this:

```json
{
  "candidate_id": "repair-1",
  "target": {
    "node_id": "hole-5",
    "span": {
      "file": "src/order.veln",
      "start": { "line": 42, "column": 12 },
      "end": { "line": 42, "column": 25 }
    }
  },
  "edit_summary": "Replace hole with parsed order total",
  "reason": "expected_type_match",
  "evidence": [
    { "kind": "type", "status": "passed" },
    { "kind": "satisfy", "status": "valid_unknown" },
    { "kind": "test", "status": "not_run" }
  ],
  "blocking_obligations": ["satisfy.valid_unknown", "test.not_run"],
  "verification_hint": "veln test tests/order_test.veln",
  "application_policy": "manual_review_required"
}
```

## Open Details

The final candidate schema, edit representation, ranking model, command name,
and user-confirmation protocol remain open. This result only settles the
semantic boundary: candidate generation is advisory evidence, while edit
application needs explicit gates.

The first implementation also does not need to solve patch minimality or
semantic equivalence. It only needs to avoid labeling candidates as safe when
the available evidence is incomplete.

## References

- Le Goues, C., Nguyen, T., Forrest, S., & Weimer, W. (2012). GenProg: A
  Generic Method for Automatic Software Repair. *IEEE Transactions on Software
  Engineering*, 38(1), 54-72. https://doi.org/10.1109/tse.2011.104
- Monperrus, M. (2018). Automatic Software Repair: A Bibliography. *ACM
  Computing Surveys*, 51(1), 1-24. https://doi.org/10.1145/3105906
- Qi, Z., Long, F., Achour, S., & Rinard, M. (2015). An analysis of patch
  plausibility and correctness for generate-and-validate patch generation
  systems. *ISSTA 2015*, 24-36.
  https://doi.org/10.1145/2771783.2771791
- Long, F., & Rinard, M. (2016). Automatic patch generation by learning correct
  code. *POPL 2016*, 298-312.
  https://doi.org/10.1145/2837614.2837617
- Microsoft. (2026). *Language Server Protocol Specification - 3.17:
  Diagnostic*.
  https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#diagnostic
- Fanning, M. C., & Golding, L. J. (Eds.). (2020). *Static Analysis Results
  Interchange Format (SARIF) Version 2.1.0*. OASIS Standard.
  https://docs.oasis-open.org/sarif/sarif/v2.1.0/os/sarif-v2.1.0-os.html

## Consequence

Veln can expose repair guidance early without freezing an unsafe automatic
repair workflow. Agents get structured candidates and verification hints, while
the language keeps a conservative boundary between "worth trying" and
"authorized to apply".
