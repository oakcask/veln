---
role: proposal
update-when: Static contract syntax, predicate-fact or callable-type prerequisite state, body and summary verification, boundary validation, generated API documentation, extension requirements, or acceptance gates change.
---

# Static Contracts for Composable Calls

Enable humans and coding agents to compose functions from their published
contracts. A caller proves the callee's precondition. The implementation proves
its postcondition under that precondition. The caller can then use the
postcondition without inspecting the implementation.

This is a design proposal, not implemented behavior or a Ready implementation
target. All examples with `static` use proposed syntax. The initial scope is
immutable scalar facts and contract-bearing function types. General refinement
type aliases, arithmetic verification, and stateful specifications are later
work. The acceptance tables below are planned evidence, not passing tests.

## Existing behavior and prerequisite investigation

The investigation uses the `origin/main` base revision `9ea04547d`. Current
behavior is owned by [Contracts](../specification/contracts.md) and
[Types](../specification/types.md), not by this proposal or older decision
records. The separate prerequisite proposals below own the shared predicate
and callable-type changes; this page owns static enforcement and proof trust.

| Inspected boundary | Current evidence | Consequence for this proposal |
| --- | --- | --- |
| Contract validation and discharge | `contracts.md` classifies valid obligations as `statically_proven` or `runtime_required`; even a false valid predicate can remain runtime-required. `crates/veln-sema/src/contracts.rs` reasons over predicate text. | Existing static classification is an optimization for executable contracts, not proof of a call's admissibility. Add an explicit static obligation mode. |
| Callable identity | `Type::Function` in `crates/veln-sema/src/semantic_model.rs` and `FunctionSignature` in `crates/veln-sema/src/types/signatures.rs` contain parameter/result types and effects but no contract. `FunctionSignature::ty` constructs a contract-free callable type. | Contracts need binder-aware callable metadata that survives aliases, imports, aggregates, and indirect calls. Remembering only the original declaration cannot satisfy higher-order use. |
| Control flow | `crates/veln-sema/src/analysis/body/control_flow_results.rs` checks Boolean conditions and joins branch result types. Existing ADT refinement supports stable places and scoped aliases. | Reuse stable-place identity where suitable, but add predicate facts and exit-sensitive flow. ADT variant membership alone does not establish integer inequalities. |
| Boolean evaluation | `emit_binary` in `crates/veln-backend-jvm/src/classfile/pattern_and_value_emission.rs` and `emit_contract_binary` in `crates/veln-backend-jvm/src/classfile/contract_emission.rs` evaluate both operands, left before right. | Preserve eager evaluation in ordinary expressions and executable predicates. A Boolean operand is not a conditional execution guard for its sibling. |
| Implementation verification | `check_body` in `crates/veln-sema/src/analysis/body.rs` validates clauses before checking body lines. | Predicate validity does not show that every return meets an `ensure`. Add return obligations and call-summary reasoning. |
| Lowering | `lower_contracts` in `crates/veln-sema/src/lowering/function_lowering.rs` independently classifies text. `CoreContract` in `crates/veln-core/src/model.rs` stores a string and the two existing statuses. | Checked static proof results must reach lowering. Lowering must not independently promote an unproved static claim. |
| Numeric meaning | `contracts.md` includes exact rational reasoning and special runtime-`Int` rules for remainder-bearing expressions; `static_literals.rs` contains wrapping integer evaluation. | Do not reuse mathematical simplifications as a sound fixed-width verifier without a semantic audit. First admit comparisons, not arithmetic terms. |

### Prerequisite decisions

Two prerequisite language changes have separate proposal authorities:

| Prerequisite | Owned change | Dependency |
| --- | --- | --- |
| [Typed Predicate Logic and Flow Facts](typed-predicate-facts.md) | Typed scalar logic, value identity, flow transfer, and bounded entailment outcomes. | No dependency on static-clause syntax, callable contracts, or a new numeric/effect model. |
| [Contract-bearing Callable Types](contract-bearing-callable-types.md) | Bound pre/post interfaces in function types, assignment, joins, and public retention. | Uses Typed Predicate Logic and Flow Facts for predicates, substitution, and implication. |

The dependency order is predicate facts, then callable interfaces, then the
complete static-contract feature. The first two proposals own P1–P3 language
semantics. P4–P5 remain here because body verification, summary trust, static
failure, and publication define this feature itself. Internal AST/core changes
and tool projections are implementation obligations, not additional language
proposals.

All three proposals remain Design In Progress. A prerequisite prototype may
supply feasibility evidence, but must not expose unchecked callable promises
or claim the complete feature. No new backend, general dependent types,
ownership system, arithmetic model, or SMT dependency is required for this
comparison-based slice.

| Gate | Required design change | Evidence before implementation selection |
| --- | --- | --- |
| P1: typed logic | Integrate the typed logic from [Typed Predicate Logic and Flow Facts](typed-predicate-facts.md). | Reviewed independent model and consumer obligations with resolved identities and runtime integer meaning. |
| P2: flow facts | Integrate the transfer and reachability rules from the predicate-fact prerequisite. | Consumer call/return cases using branch, record, alias, eager-operand, and unreachable-path facts. |
| P3: callable contracts | Integrate [Contract-bearing Callable Types](contract-bearing-callable-types.md). | Reviewed callable syntax/type evidence and direct, imported, nested, and indirect static-call cases. |
| P4: proof trust | Verify source implementations, define recursive summary use, and quarantine invalid declarations. Define which generated or host-backed declarations can supply facts. | Positive/negative body and recursive-component cases; no external summary accepted solely because its signature declares it. |
| P5: executable boundary | Define static failure outcomes, JSON/human projections, and checked-to-core handoff. Preserve the meaning of existing runtime clauses, check executable entry adapters, and retain contract modes in generated API documentation. | Call-site, return, entry-boundary, human/JSON, lowering, and generated-documentation cases. |

P1 and P2 precede proof implementation. P3 and P4 precede exposing contract
facts through general function values. P5 precedes publishing the feature.
These gates can be implemented in focused follow-up targets after their design
questions are resolved; none is currently cataloged as Ready.

Each gate must supply a reviewable contract before an implementation target is
selected. A prototype alone does not settle its observable behavior:

| Gate | Required deliverable and acceptance boundary |
| --- | --- |
| P1 | The prerequisite's typed grammar, mandatory truth/order cases, and measured deterministic work policy, exercised through static call/return obligations. |
| P2 | The prerequisite's transfer/join model, exercised through result binding, scope exit, records, unreachable calls/returns, and loop boundaries in consumer programs. |
| P3 | The callable prerequisite's grammar, binder, assignment, contextual, and join evidence, plus static-clause declaration grammar and consumer calls that cannot bypass the retained interface. |
| P4 | A dependency-validation and recursive-component publication model. State which source snapshot and checked contracts a summary depends on. Exercise failure and recovery after a body-only change, not only a signature change; no caller may reuse an invalidated proof. |
| P5 | Diagnostic identifiers, closed JSON detail schemas and related-note kinds, adapter projections, generated-documentation mode fields and version changes, and a checked-to-core readiness invariant. Exercise atomic rejection, once-only evaluation, and preservation of existing runtime clauses. |

The P1–P3 rows are integration gates referencing the two prerequisite
proposals; they are not second authorities for their language rules. P4–P5
remain owned here. The gate deliverables are still required evidence, not
completed implementation or permission to select a partial feature that
erases contracts. Extensions
outside these gates follow the [extension requirements](#extension-requirements).

## Proposed source surface and guarantee

Use an explicit modifier to request a static guarantee. This proposal chooses
opt-in syntax as a starting design decision; it does not assume that the prior
conversation settled the eventual default for all contracts:

```veln
fn consume(value: Int) -> Int
    require static (value > 0)
    value
end

fn normalize(value: Int) -> result: Int
    ensure static (result > 0)
    if value > 0
        value
    else
        1
    end
end

fn composed(input: Int) -> Int
    consume(normalize(input))
end
```

The caller of `consume` must prove the requirement. Every ordinary return from
`normalize` must prove the postcondition. Its result therefore supplies the
fact needed by `consume`, including across modules.

| Clause | Caller obligation | Implementation obligation | Unknown result |
| --- | --- | --- | --- |
| `require static (P)` | Prove `P` at each call using actual argument values. | May assume `P` while checking the body. | Compile-time error at the call. |
| `ensure static (Q)` | May use `Q` after the call returns. | Prove `Q` for each returned value under static requirements and path facts. | Compile-time error at the return. |
| Existing unmodified clauses | Existing runtime contract behavior. | Existing runtime contract behavior. | Existing runtime fallback; no static summary fact. |

Multiple static requirements form a conjunction. Multiple static ensures form
a conjunction. A missing static requirement or ensure means `true` in that
position. Unmodified `require`, `ensure`, and `invariant` remain executable
contracts; they do not silently become static guarantees. `invariant static`
is outside the initial slice. Static body proofs cannot assume unmodified
runtime clauses. Mixing modes does not promise absence of unrelated runtime
contract failure.

A static clause has no runtime predicate evaluation. Failure to prove it never
inserts an implicit check or silently changes its mode. The static guarantee
is conditional on successful compilation of the complete checked dependency
set. It guarantees the marked boundary facts, not freedom from all runtime
errors.

The guarantee is partial correctness. A postcondition applies when a call
returns a value. It does not prove termination, progress, cancellation behavior,
or absence of abrupt failure. A `?` propagation is a return of the enclosing
function's error result and must satisfy that function's static ensures; it
is not a success result to which success-only facts can be attached.

### Callable interface dependency

[Contract-bearing Callable Types](contract-bearing-callable-types.md) owns
callable contract syntax, binder scope, alpha-equivalence, assignment, and
joins. A complete static-contract feature must retain those interfaces through
aliases, aggregates, imports, and indirect calls. A direct-call-only prototype
does not satisfy this proposal's compositional guarantee.

## Predicate and flow dependency

[Typed Predicate Logic and Flow Facts](typed-predicate-facts.md) owns the
admitted proof language, value identities, record and expression-result fact
transfer, eager operand timing, reachability, and entailment outcomes. This
proposal supplies validated static requirements as entry assumptions and
verified static ensures as call-result facts to that model. Neither existing
runtime clauses nor unchecked summaries supply those facts.

### External inputs

Ordinary runtime branching is the initial bridge from unknown input to a
proved call. It does not require a proof object or a new assertion primitive:

```veln
fn accept_input(value: Int) -> Result<Int, String>
    if value > 0
        Ok(consume(value))
    else
        Err("expected a positive value")
    end
end
```

Calling `consume(value)` before the branch is rejected. The false branch
retains no positive fact. A validator helper can return a Boolean with a
verified equivalence postcondition; branching on that result establishes the
same facts. A helper with only a comment or an unmodified runtime ensure does
not provide such a summary.

A `Result<PositiveInt, Error>` validator and arbitrary refined scalar storage
are useful later extensions. They require a refinement carrier/construction
rule not present in the initial design. This proposal does not pretend that
`Result<Int, Error>` alone preserves a positive payload fact after matching.

## Verification, recursion, and trust

Each body is checked under its static requirements. Every reachable call,
including a call appearing inside an executable runtime predicate, must
discharge the callee's static requirements. Every reachable returned value
must discharge the body's static ensures. Static summaries never depend on
declaration order or on whether a caller can access the source body.

Recursive and mutually recursive bodies may use declared summaries as partial
correctness hypotheses. Each member must pass its own call and return
obligations. No member's ensures are assumed at its own entry. A failed member
invalidates publication of the dependent component. Non-returning recursion
cannot establish a reachable returned value or justify unfolding a logical
predicate. Loop fact availability follows the
[iteration rules](typed-predicate-facts.md#iteration-boundaries). An obligation
needing an unavailable invariant reports unknown in the first slice; explicit
loop invariants and termination checking follow the extension requirements.

Public summaries reference only bound parameters/results and accessible type
identities. Internal helpers and local bindings cannot leak into public
contracts. Imported summaries are accepted only as part of a validated checked
source dependency. Contract changes invalidate dependent checking even if
ordinary parameter/result types stay the same. Hosts, generated functions, and
binary-only packages receive no new trusted static summaries in the first
slice. A later trusted-summary mechanism needs explicit conformance evidence.
Ordinary host-returned `Int` values can still be validated by branching.

Compiler-generated executable entries and callback adapters obey the same call
rule. Invoking a function as a run/test entry does not assume its static
requirements. If an adapter cannot establish them, executable preparation is
rejected. An externally callable adapter must validate unknown input before
making the static call. An unchecked external ABI cannot publish a static-only
entry guarantee. The initial slice adds no automatic dynamic wrapper.

Holes and invalid or recovery-typed declarations never mint proven facts.
Tools may show provisional obligations for partial programs, but no executable
or published verified interface is produced while static obligations remain
unresolved. A repair that deletes a requirement or changes it to a runtime
clause changes the API guarantee; tools must not present it as proof of the
original obligation.

## Failure and tool contract

| Proof outcome | Compilation | Required observation |
| --- | --- | --- |
| Proven | Accept this obligation. | Static mode and proven outcome are retained through checked lowering. |
| Vacuously proven | Accept a valid obligation on a proved unreachable path. | Retain the unreachable-path basis in checked proof evidence; no value or fact is exported to a reachable path. |
| Refuted under the path facts | Reject. | State the failed predicate at the call or return. A concrete counterexample is optional and must be valid if shown. |
| Unknown or proof budget exhausted | Reject. | Distinguish inability to prove from a demonstrated violation. Never label a timeout a counterexample. |
| Unsupported logic or invalid predicate | Reject. | Identify the unsupported construct or ordinary syntax/name/type failure; add no assumptions. |

Entailment and vacuity have the meanings defined by the
[predicate-fact outcome model](typed-predicate-facts.md#entailment-outcomes).
This proposal maps unresolved obligations to static compilation errors;
the shared model alone does not change the existing runtime fallback.

Human output places the failed fact in the primary message at the call,
assignment, or return. Related notes identify the declared requirement/ensure,
relevant assumptions, and a justified repair hint. For example, the primary
message can say that `value > 0` is not established at this call. It must not
claim that `value` is nonpositive merely because proof is unknown.

JSON diagnostics expose the obligation kind, static mode, substituted
predicate, proof outcome, reason, and source-related locations. Concrete field
names and diagnostic identifiers are a P5 schema decision. CLI, LSP, MCP, and
repair views must project the same result. A suggested guard must preserve a
failure path chosen by the program author; tools cannot invent an application
error policy. Proof work limits are deterministic; wall-clock interruption can
fail checking but cannot change unknown into proven.

## Generated API documentation

Generated documentation is part of the published contract consumed by humans
and coding agents. The current [package documentation boundary](../specification/package-documentation.md#published-boundary)
already includes public function contracts. Static mode must remain observable
in both generated Markdown and package-documentation JSON:

- Markdown displays each public requirement and ensure with its predicate and
  explicit static mode, for example `require static (value > 0)`. Existing
  executable clauses remain distinguishable from static clauses. A reader
  must not need the implementation body to identify the caller's obligation.
- JSON retains clause kind and predicate text and provides an explicit
  machine-readable contract mode. Mode must not be inferred by parsing display
  text. The precise schema and generator-contract version change belong to P5.
- Rendered public signatures and JSON preserve contracts nested in exposed
  function types, including callback parameters, results, and record fields.
  Binder names are scoped to their callable; documentation cannot erase a
  requirement while retaining only ordinary input and result types.
- Public documentation exposes no private proof assumptions or implementation
  locals. Generation does not label unresolved or invalid static contracts as
  verified. The catalog's semantic gate rejects such publication without a
  partial successful catalog.
- Changing only a public contract's mode or predicate changes the canonical
  documentation content and its `doc-digest`. Retained MCP declaration
  resources and rendered Markdown must agree with the catalog's contract mode
  and predicate. Updating a cached view cannot keep a superseded guarantee.

These are proposed extensions to generated documentation, not claims that the
current generator understands `static`. Documentation communicates the same
interface used by checking; it is not independent proof that the implementation
satisfies a contract.

## Related work and alternatives

Primary documentation was inspected for the following design comparisons.
These sources explain mechanisms, not measured productivity improvements for
AI or humans. Shorter repair loops and lower context requirements are design
hypotheses to evaluate, not established benchmark results.

| Source and inspected material | Finding and applicability | Design consequence |
| --- | --- | --- |
| F* project, [Getting off the ground: Boolean refinement types and refinement subtyping](https://fstar-lang.org/tutorial/book/part1/part1_getting_off_the_ground.html#boolean-refinement-types) | Refinements restrict values using predicates; introduction requires proving the substituted predicate. F* also uses unbounded mathematical integers, unlike Veln's runtime integer representation. | Adopt substitution-based obligations and callable result relations. Do not copy mathematical arithmetic rules or require full dependent programming. |
| LiquidHaskell project, [Spec Reference: Abstract and Bounded Refinements](https://ucsd-progsys.github.io/liquidhaskell/specifications/#abstract-and-bounded-refinements), [Termination Metrics](https://ucsd-progsys.github.io/liquidhaskell/specifications/#termination-metrics) | Refinement subtyping induces implication constraints. Measures and termination metrics are separate specification mechanisms. The host language and proof infrastructure differ from Veln. | Start with a restricted logic and explicit summaries. Defer abstract refinements, collection measures, and arbitrary function reflection; purity alone is insufficient proof authority. |
| Dafny community, [Reference Manual: Arrow types](https://dafny.org/dafny/DafnyRef/DafnyRef.html#sec-arrow-subset-types) and [Subset types](https://dafny.org/dafny/DafnyRef/DafnyRef.html#sec-subset-types) | Function types distinguish precondition-bearing functions and heap-reading boundaries. Subset-type introduction has a verification obligation. Veln does not currently provide Dafny's verification or heap model. | Retain requirements through function values and reject erasure to a callable accepting all inputs. Defer heap predicates and arbitrary refined value types. |

The existing [static/runtime boundary rationale](../reference/source-decisions/records/result-contract-static-runtime-boundary.md)
explains why executable contracts were useful before a general verifier.
This proposal adds a strict static mode instead of treating that rationale's
future possibilities as implemented capabilities. The current specification,
which has only proven/runtime-required classification, remains authoritative.

| Alternative | Decision and reason |
| --- | --- |
| Make every existing `require` static immediately | Defer. Existing predicates permit pure calls beyond this slice's logic. Explicit mode makes unsupported proof requirements visible without removing executable contracts. This is a usability decision, not a backward-compatibility requirement. |
| Automatically fall back to runtime checks | Reject for marked static contracts. A successful compile would no longer mean that the caller met the requested static guarantee. |
| Check only named direct calls | Insufficient as the final target. Erasing a requirement through a callback undermines composition. A direct-call prototype may supply gate evidence but cannot publish the complete feature. |
| Introduce full refinement types and an SMT solver first | Defer. The comparison-only slice can test the user-facing contract without selecting a dependency or proof-assistant workflow. Solver adoption requires its own dependency and numeric-soundness review. |
| Use nominal validated wrappers for every condition | Useful alternative for independent reusable properties. Cross-argument and result/input relations still need contract logic. General carriers remain later work. |

## Extension requirements

The initial slice remains comparison-based and monomorphic. The following
rows define the requirements for reconsidering excluded capabilities; they do
not select them for implementation or require completing them before the
initial slice. Each later proposal must identify a concrete API need, inspect
relevant primary sources, choose a complete observable boundary, and provide
positive, negative, and boundary acceptance cases. Unsupported uses retain
their explicit error or unknown outcome until that extension is implemented.

| Deferred capability | Reconsider when | Required contract and adoption evidence |
| --- | --- | --- |
| Integer arithmetic and bitwise proof terms | An API needs a relation such as a size increment or an index calculation. | Define each admitted operator using runtime signed 64-bit behavior, including wrapping, division, remainder, invalid shifts, and abrupt failure. Distinguish operation-definedness obligations from value claims. Check endpoints against an independent runtime-semantic model; never import unbounded-integer identities silently. |
| Float predicates | A concrete numeric API needs static Float facts. | Model NaN, infinities, signed zero, comparisons, and any admitted arithmetic. Demonstrate rejected total-order inferences and agreement with supported backends. An Int proof rule cannot be reused merely because the source operator is shared. |
| Collection measures and ADT payload relations | An API needs length/index or success-payload guarantees. | Define which values a measure observes, its numeric range, and verified construction, pattern, and update rules. Prove that `Ok`-branch facts do not attach to `Err`, including `?` propagation. Do not infer a positive payload from `Result<Int, Error>` alone. |
| Reusable refinement carriers and aliases | A validated property must survive storage and public API transfer. | Specify introduction, elimination, assignment, visibility, and invalidation rules. Invalid construction must fail without publishing a value carrying the property. Check imported aliases, containers, pattern extraction, and attempts to forge or erase the guarantee. |
| Generic, variadic, and effect-row-polymorphic contracts | A reusable higher-order API cannot express its contract with monomorphic fixed-arity types. | Define binder scope, substitution, variance, row constraints, and the meaning of contracts over a variadic tail. Distinguish ordinary containers holding monomorphic callables from abstraction over predicates. Check nested callbacks and failed substitutions without leaking tentative facts. |
| Loop invariants and termination | A loop needs facts unavailable from outer immutable values and its current guard, or an API needs a termination promise. | Separate partial correctness from total correctness. For invariants, verify initialization, preservation on every backedge, and facts on normal and early exits. A termination extension additionally needs a well-founded decrease on every relevant recursive/loop edge. Retain partial-correctness behavior for unmarked functions. |
| Logical predicates, reflection, and lemmas | Repeated contracts need named logical abstraction beyond a verified Boolean-returning helper. | Define logical admissibility, substitution, and proof authority independently of executable purity. Reflection must establish totality and reject circular proof authority; lemmas must discharge their own obligations. Check divergent pure functions and mutually dependent unproved lemmas. |
| Mutable-state and host-state predicates | A contract must describe observations that can change after a call or guard. | Define state snapshots, reads/writes or frame conditions, aliasing, concurrency, and invalidation across calls and suspension. Demonstrate that a checked guard cannot justify a later observation after an invalidating change. A scoped fake handler is test support, not authority for a production summary. |
| Trusted host, generated, and binary-only summaries | A dependency must expose verified facts without checked source bodies. | Define the trust root, summary identity, conformance evidence, and failure/revocation behavior. Bind summaries to the dependency and semantic versions they describe. Reject mismatched or invalidated artifacts and test transitive dependent rechecking; a signature alone remains insufficient. |
| Dynamic boundary wrappers | An external entry must accept unknown inputs while calling a static API. | Define which supported predicates can run as checks, once-only input evaluation, and the author-selected failure policy. State what remains a static guarantee inside the wrapper. Do not silently change a static clause to runtime mode or invent an application error policy. |
| Solver adoption | Required contracts exceed the bounded comparison engine. | Review the dependency and runtime numeric encoding, supported theories, deterministic work accounting, unknown/timeout behavior, and independent soundness cases. Compare proof coverage and resource cost on named workloads. Solver success cannot authorize unsupported terms or unvalidated summaries. |
| Static invariants and default contract mode | Concrete usage shows that explicit requirements/ensures leave a necessary boundary unexpressed or systematically mislead users. | Define the covered entry/exit events and mode spelling, generated documentation, diagnostics, and repair consequences. Evaluate migration of executable pure-call predicates and mixed modes. Changing a default must not silently reinterpret an unsupported predicate as proved or erase a caller obligation. |

These rows bound future design work, not a commitment to adopt every
capability. Close or split an extension when its concrete use case is rejected
or moves to another proposal; do not leave it as an implied initial guarantee.

## Planned acceptance evidence

These cases specify outcomes independently of the chosen proof algorithm.
The names below identify planned cases, not existing fixture directories.
They exercise the two prerequisite models through static-contract consumers;
the owning prerequisite pages define the shared logic and type rules.

| Case | Input or change | Expected outcome |
| --- | --- | --- |
| Guarded call | Inside `if x > 0`, call a function requiring `x > 0`. | Accept the true branch; reject the same call before the guard and in the false branch. |
| Propagated assumption | Caller requires `x > 0`, passes an immutable alias of `x`. | Accept; a shadowed or unrelated same-spelled value supplies no fact. |
| Continuing path | The nonpositive branch returns an error, then the continuation consumes `x`. | Accept on the continuing path only. |
| Producer/consumer | `normalize` above feeds `consume`, directly and through an imported alias. | Accept without inspecting the producer's body in the caller. |
| Body failure | Change `normalize`'s fallback from `1` to `0`. | Reject at that return and invalidate its dependent static summary. |
| Relationship | A maximum-selection function returns either `a` or `b` under the corresponding ordering branch; promises result is at least both. | Accept using branch comparisons and returned-value equality. |
| Named validator | A Boolean helper proves `result == (x > 0)`; caller branches on its result. | Accept consumption in the true branch; a runtime-only helper promise is insufficient. |
| Eager Boolean operands | A Boolean helper requires `x > 0`; call it on the right of `x > 0 and ...` or `x <= 0 or ...`, in an ordinary condition and in a runtime predicate. | Reject for unconstrained `x`; accept under an enclosing positive guard. A later comparison cannot justify an earlier call. Preserve evaluation of both operands. |
| Operand summaries | The left operand returns a Boolean and independently ensures `x > 0`; the right operand consumes `x`. | Accept using the verified ensure, without assuming the left Boolean is true. A mere equivalence between that Boolean and positivity is insufficient before branching. |
| Record fact transfer | Store `normalize(x)` in a nested immutable record, alias the record, destructure its field, and consume each corresponding scalar. | Accept all retained projections, including through compatible width annotations. A different field or root without a value relation supplies no proof. Check returned-record ensures as well. |
| Expression result join | Bind the positive branches of `normalize` through `if` or `begin` before consuming the result; change one reachable branch to zero. | Accept the all-positive join and preserve output facts after inner locals leave scope; reject the changed branch. A noncontinuing branch supplies no result contribution. |
| Unreachable obligation | Call `consume(0)` under both `x > 0` and `x <= 0`, or under `if false`; then call it on a reachable continuation. | The unreachable call is vacuously proven; the reachable call is refuted. No contradictory fact leaks into the continuation. |
| Vacuous declaration | Declare contradictory requirements with `ensure static (false)`, then attempt to call it or assign it to an all-input callable on a reachable path. | Accept the well-formed declaration under partial correctness; reject the reachable call and requirement erasure. A hole, invalid name/type, or unsupported static predicate in dead code still rejects compilation. |
| Non-returning summary | Use a valid checked `ensure static (false)` summary, then contrast it with an ordinary recursive call lacking that summary. | Only the verified summary establishes no normal continuation; neither case proves termination or creates a returned value. |
| Iteration boundary | A fact about a newly read value is established late in one iteration and would be needed before the next read is guarded. | Reject that use; the prior iteration's fact is unavailable. Accept a call proved independently by each iteration's guard or unchanged outer facts. |
| Callback preservation | Store a positive-input callable in a record or pass it to another function. | Retain the requirement; reject assignment to a plain all-input callable. Accept weaker source requirements and stronger source ensures when implication is proved. |
| Alpha-equivalence | Rename callable parameter/result binders without changing relations. | Preserve assignability; imports with equal spellings but different identities remain distinct. |
| Repeated computation | Compare `read()` with zero, then consume a second `read()` result. | Reject unless that second value has its own proof. Guarding a saved immutable result is sufficient. |
| Numeric boundary | Attempt arithmetic in a static predicate, or prove `x + 1 > x` from an unconstrained body result. | Unsupported predicate error / unknown return proof; never accept by mathematical-integer simplification. Include maximum/minimum `Int` comparisons. |
| Return coverage | A normal return and a `?` error return disagree with a static ensure. | Check both enclosing-function result paths; reject the failing path. |
| Recursion | A mutually recursive component contains a failing base-case return. | Reject the component; declaration order cannot make its summary trusted. |
| Boundary trust | Host return, hole, invalid imported body, or generated declaration supplies an unverified promise. | No static fact; ordinary guarded host values remain usable. |
| Entry adapter | Select an executable entry whose static requirement the generated caller cannot establish. | Reject executable preparation; selecting an entry does not bypass a call obligation. An explicit guarded adapter can call it. |
| Obligation outcome | A false literal call, an unconstrained call, `x >= 0` with an obligation `x > 0`, and an exhausted search. | Distinct refuted/unknown/budget reasons; mixed satisfying/violating inputs are unknown, not universally refuted. No case compiles through implicit runtime fallback. |
| Mode separation | Remove `static` from a clause using a currently accepted runtime predicate. | Existing executable contract behavior remains; it supplies no verified static summary. |
| Generated documentation | Export a function mixing static and runtime clauses and an API with a contract-bearing callback; then change only a clause mode or predicate. | Markdown, catalog JSON, and MCP declaration resources retain the distinction and nested contracts. The documentation digest changes. Private assumptions stay absent; an invalid static summary cannot produce a successful catalog. |
| Incremental invalidation | Strengthen an imported requirement while keeping ordinary types unchanged. | Recheck callers and report newly unproved calls; no stale cache acceptance. |
| Body-only invalidation and recovery | Change an imported producer body to violate its unchanged ensure; check consumers and generated documentation, then restore the body. | Reject consumers/publication while the summary is invalid and recover after revalidation. Warm and fresh checking agree, including for a mutually recursive component. |

Planned verification ownership:

- Syntax, formatting, and binder scoping: parser/AST/type tests, accepted and
  rejected source-surface fixtures, and formatter round trips.
- Entailment and path facts: focused `veln-sema` tests using the truth/order
  model; cover success, boundary, rejection, vacuity, record projections,
  expression-result joins, iteration boundaries, and restoration after scope
  exit.
- Observable compiler behavior: `examples/specification/check/` cases for
  direct/indirect/imported calls, human related notes, JSON outcomes, and holes.
- Runtime preservation and checked lowering: focused `run/` and `test/`
  toolchain cases for the compiler/backend contract, including existing runtime
  mode, eager Boolean operands, and once-only evaluation. These are not stdlib
  API tests. Observable evaluation cases must detect a skipped, duplicated,
  or reordered operand in ordinary code and predicates that remain
  runtime-required. Existing statically proven executable clauses may still
  omit their runtime checks as specified by the current contract mode.
- Future stdlib adoption: API behavior belongs in Veln companion tests under
  `crates/veln-stdlib/veln/`, following the
  [test placement policy](../reference/toolchain-test-harness.md#test-placement-policy).
- Generated API documentation: `examples/specification/doc/` cases for Markdown
  and JSON, package-catalog identity tests for mode/predicate-only digest
  changes, and MCP declaration-resource cases for retained content. Cover
  nested callable contracts, mixed modes, private-name exclusion, and atomic
  failure for an invalid static summary.
- Incremental checking: language-service tests that change only an imported
  static contract or its implementation body and compare saved diagnostics with
  a fresh check, including invalidation and recovery of documentation views.

Before Ready selection, apply the predicate prerequisite's
[bounded-work acceptance model](typed-predicate-facts.md#planned-acceptance-and-verification)
to repeated call summaries and consumer programs as well as isolated queries.
Wall-clock samples are supporting evidence on a controlled runner, not a
portable acceptance threshold. Run generated/analysis workloads through
`bash scripts/agent-run` and the repository performance-audit workflow.

## Readiness and lifecycle

This proposal stays under Design In Progress until its two prerequisite
language proposals are resolved and P1–P5 have precise grammar,
mandatory proof rules, trust boundaries, diagnostic schemas, work limits, and
reviewed feasibility evidence. In particular, callback contract joins, `?`
return facts, eager operands, record fact transfer, unreachable-path obligations,
and invalid recursive-component publication must be exercised before promising
compositional guarantees. The separate predicate-fact and callable-interface
proposals are prerequisites; ownership/effect and numeric-language redesigns
are not prerequisites for the restricted slice.

A later Ready target must select a complete externally observable slice and
its acceptance cases. During implementation, promote checked examples and
current behavior into `contracts.md`, `types.md`, and the relevant grammar,
diagnostic, editor, package-interface, and generated-documentation
specifications. Update their `update-when` dependents when the corresponding
behavior changes. The current
specification and examples need no behavior edits for this documentation-only
proposal. Promote completed prerequisite behavior from its owning proposal
and remove that proposal's catalog entry when its remaining scope is closed.
Replace implemented prerequisite links here with the current specification;
do not retain a completed proposal merely as a dependency route. Remove
completed proposal sections and retire this page when all remaining planned
work is either implemented or explicitly closed.
