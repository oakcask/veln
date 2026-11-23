---
role: proposal
update-when: Static contract syntax, proof rules, callable contract subtyping, boundary validation, prerequisite decisions, or acceptance gates change.
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
records. No existing active proposal in the catalog owns this change.

| Inspected boundary | Current evidence | Consequence for this proposal |
| --- | --- | --- |
| Contract validation and discharge | `contracts.md` classifies valid obligations as `statically_proven` or `runtime_required`; even a false valid predicate can remain runtime-required. `crates/veln-sema/src/contracts.rs` reasons over predicate text. | Existing static classification is an optimization for executable contracts, not proof of a call's admissibility. Add an explicit static obligation mode. |
| Callable identity | `Type::Function` in `crates/veln-sema/src/semantic_model.rs` and `FunctionSignature` in `crates/veln-sema/src/types/signatures.rs` contain parameter/result types and effects but no contract. `FunctionSignature::ty` constructs a contract-free callable type. | Contracts need binder-aware callable metadata that survives aliases, imports, aggregates, and indirect calls. Remembering only the original declaration cannot satisfy higher-order use. |
| Control flow | `crates/veln-sema/src/analysis/body/control_flow_results.rs` checks Boolean conditions and joins branch result types. Existing ADT refinement supports stable places and scoped aliases. | Reuse stable-place identity where suitable, but add predicate facts and exit-sensitive flow. ADT variant membership alone does not establish integer inequalities. |
| Implementation verification | `check_body` in `crates/veln-sema/src/analysis/body.rs` validates clauses before checking body lines. | Predicate validity does not show that every return meets an `ensure`. Add return obligations and call-summary reasoning. |
| Lowering | `lower_contracts` in `crates/veln-sema/src/lowering/function_lowering.rs` independently classifies text. `CoreContract` in `crates/veln-core/src/model.rs` stores a string and the two existing statuses. | Checked static proof results must reach lowering. Lowering must not independently promote an unproved static claim. |
| Numeric meaning | `contracts.md` includes exact rational reasoning and special runtime-`Int` rules for remainder-bearing expressions; `static_literals.rs` contains wrapping integer evaluation. | Do not reuse mathematical simplifications as a sound fixed-width verifier without a semantic audit. First admit comparisons, not arithmetic terms. |

### Prerequisite decisions

These changes are necessary parts of the proposal. They do not require a new
language backend, general dependent types, a new ownership system, or an SMT
dependency before the initial slice can be attempted.

| Gate | Required design change | Evidence before implementation selection |
| --- | --- | --- |
| P1: typed logic | Resolve each logical name to a declaration/binder and retain operand types, source spans, and signed integer meaning. Distinguish facts about values from repeated source spellings. | A small independent truth/ordering model and tests for shadowing, argument substitution, minimum/maximum integers, and rejection of unsupported numeric terms. |
| P2: flow facts | Define facts for immutable locals, parameters, stable record fields, branch polarity, continuing paths, and return values. | Branch/alias/early-exit acceptance cases, including failures for leaked and unrelated facts. |
| P3: callable contracts | Add bound parameters/results to callable contracts, define alpha-equivalence and implication-based assignment, and retain contracts across public interfaces. | Grammar/formatter prototype and direct, imported, aliased, nested, and indirect callable cases. |
| P4: proof trust | Verify source implementations, define recursive summary use, and quarantine invalid declarations. Define which generated or host-backed declarations can supply facts. | Positive/negative body and recursive-component cases; no external summary accepted solely because its signature declares it. |
| P5: executable boundary | Define static failure outcomes, JSON/human projections, and checked-to-core handoff. Preserve the meaning of existing runtime clauses and check executable entry adapters. | Call-site, return, entry-boundary, human/JSON, and lowering cases. |

P1 and P2 precede proof implementation. P3 and P4 precede exposing contract
facts through general function values. P5 precedes publishing the feature.
These gates can be implemented in focused follow-up targets after their design
questions are resolved; none is currently cataloged as Ready.

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

### Contract-bearing callable types

A candidate type spelling is:

```text
(fn(value: Int) -> result: Int
    require static (value > 0)
    ensure static (result > 0))
```

Names in this form bind within the callable type. Parameter renaming does not
change type identity. Outer parentheses delimit the complete callable type in
records, generic arguments, and nested functions. Declaration clauses and type
clauses denote the same pre/post pair. Effect rows retain their existing role.
The spelling is subject to the P3 grammar/formatter gate; it is not an accepted
source grammar. The grammar prototype must settle clause ordering, nesting,
line breaks, and the interaction with effect rows before Ready selection.

For compatible ordinary input/result types and effects, let a source callable
have `(Ps, Qs)` and a target callable have `(Pt, Qt)`. Assignment requires:

- `Pt` implies `Ps`: the source cannot demand more than the target allows.
- Under `Pt`, `Qs` implies `Qt`: the source must deliver the target's promise.

The existing input contravariance, result covariance, and effect compatibility
rules also apply. Logical implication failure or unknown implication rejects
assignment. A source requiring `x > 0` cannot become plain `fn(Int) -> Int`,
whose static requirement is `true`. A source requiring `x >= 0` can serve a
target requiring `x > 0`. A stronger postcondition can be forgotten; a stronger
requirement cannot be erased. Function joins retain only a contributed
callable type justified by these rules; they do not drop requirements to make
a join succeed. Otherwise an explicit compatible annotation is required.

The first slice supports monomorphic, fixed-arity callable contracts over the
admitted scalar paths. Generic proof abstraction, variadic contract binders,
and effect-row polymorphic contract entailment remain outside the slice.
Unsupported boundaries receive an explicit unsupported-static-contract error,
not a contract-free approximation.

## Initial proof language and ordinary coding

Static predicates admit:

- Boolean literals, stable Boolean paths, `not`, `and`, and `or`;
- equality/disequality of matching `Bool` or `Int` operands;
- signed `Int` ordering between literals and stable scalar paths;
- parentheses and signed integer literals within the `Int` range;
- parameter and named-result references, including immutable record fields.

A stable path denotes one immutable value, using resolved identity rather
than its displayed name. An argument expression is evaluated once according
to ordinary language evaluation; substitution refers to that value. Returned
values have fresh identities. Two calls with identical spelling do not acquire
an equality fact. Boolean aliases retain a relation to their originating
admitted condition until scope ends.

The mandatory proof fragment includes literal comparison, Boolean identities,
copy equality, equality substitution, order transitivity, weakening strict
bounds to inclusive bounds, and complementary branch conditions. The checker
need not decide every Boolean formula. More complex supported formulas can
produce an unknown proof result; bounded search must never accept them by
default. P1 must fix deterministic work limits and mandatory-fragment cases
before implementation selection.

| Program event | Facts available afterward |
| --- | --- |
| Enter a function | Static requirements over parameter values; no assumptions from its own ensures. |
| `let y = x` for an immutable scalar | Equality of these values; shadowed bindings remain distinct. |
| Enter the true/false branch of an admitted condition | The condition / its negation, scoped to that branch. |
| Join continuing branches | Only facts justified on every continuing path; an exited branch does not constrain the continuation. |
| Return a branch expression | That path's facts and its actual returned value for checking ensures. |
| Complete a call with a valid static summary | The instantiated ensures over evaluated arguments and the returned value. |
| Match an ADT | Existing variant refinement rules remain available; no arbitrary payload predicate is inferred. |

Arithmetic can still occur in ordinary bodies. In the first slice an arithmetic
result supplies no arithmetic relationship to its operands unless represented
by a separately verified admitted summary. For example, `return x + 1` cannot
justify `ensure static (result > x)`. Returning a known positive literal can
justify `result > 0`. Neither mathematical-integer identities nor Float total
ordering may be assumed. Arithmetic inside a static predicate, Float facts,
collection measures such as length, quantifiers, arbitrary predicate calls,
logical lemmas, and mutable/host state observations are unsupported.

An effect-free function is not automatically total or safe to execute in a
proof. No arbitrary user function is unfolded or executed by the proof engine.
A verified Boolean-returning function may convey a relation through an admitted
postcondition, such as `result == (value > 0)`. This supports named validation
without introducing predicate calls into the logic.

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

Each body is checked under its static requirements. Every call, including a
call appearing inside an executable runtime predicate, must discharge the
callee's static requirements. Every returned value must discharge the body's
static ensures. Static summaries never depend on declaration order or on
whether a caller can access the source body.

Recursive and mutually recursive bodies may use declared summaries as partial
correctness hypotheses. Each member must pass its own call and return
obligations. No member's ensures are assumed at its own entry. A failed member
invalidates publication of the dependent component. Non-returning recursion
cannot establish a reachable returned value or justify unfolding a logical
predicate. General loops without sufficient supported facts report unknown;
loop invariants and termination checking are later work.

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
| Refuted under the path facts | Reject. | State the failed predicate at the call or return. A concrete counterexample is optional and must be valid if shown. |
| Unknown or proof budget exhausted | Reject. | Distinguish inability to prove from a demonstrated violation. Never label a timeout a counterexample. |
| Unsupported logic or invalid predicate | Reject. | Identify the unsupported construct or ordinary syntax/name/type failure; add no assumptions. |

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

## Planned acceptance evidence

These cases specify outcomes independently of the chosen proof algorithm.
The names below identify planned cases, not existing fixture directories.

| Case | Input or change | Expected outcome |
| --- | --- | --- |
| Guarded call | Inside `if x > 0`, call a function requiring `x > 0`. | Accept the true branch; reject the same call before the guard and in the false branch. |
| Propagated assumption | Caller requires `x > 0`, passes an immutable alias of `x`. | Accept; a shadowed or unrelated same-spelled value supplies no fact. |
| Continuing path | The nonpositive branch returns an error, then the continuation consumes `x`. | Accept on the continuing path only. |
| Producer/consumer | `normalize` above feeds `consume`, directly and through an imported alias. | Accept without inspecting the producer's body in the caller. |
| Body failure | Change `normalize`'s fallback from `1` to `0`. | Reject at that return and invalidate its dependent static summary. |
| Relationship | A maximum-selection function returns either `a` or `b` under the corresponding ordering branch; promises result is at least both. | Accept using branch comparisons and returned-value equality. |
| Named validator | A Boolean helper proves `result == (x > 0)`; caller branches on its result. | Accept consumption in the true branch; a runtime-only helper promise is insufficient. |
| Callback preservation | Store a positive-input callable in a record or pass it to another function. | Retain the requirement; reject assignment to a plain all-input callable. Accept weaker source requirements and stronger source ensures when implication is proved. |
| Alpha-equivalence | Rename callable parameter/result binders without changing relations. | Preserve assignability; imports with equal spellings but different identities remain distinct. |
| Repeated computation | Compare `read()` with zero, then consume a second `read()` result. | Reject unless that second value has its own proof. Guarding a saved immutable result is sufficient. |
| Numeric boundary | Attempt arithmetic in a static predicate, or prove `x + 1 > x` from an unconstrained body result. | Unsupported predicate error / unknown return proof; never accept by mathematical-integer simplification. Include maximum/minimum `Int` comparisons. |
| Return coverage | A normal return and a `?` error return disagree with a static ensure. | Check both enclosing-function result paths; reject the failing path. |
| Recursion | A mutually recursive component contains a failing base-case return. | Reject the component; declaration order cannot make its summary trusted. |
| Boundary trust | Host return, hole, invalid imported body, or generated declaration supplies an unverified promise. | No static fact; ordinary guarded host values remain usable. |
| Entry adapter | Select an executable entry whose static requirement the generated caller cannot establish. | Reject executable preparation; selecting an entry does not bypass a call obligation. An explicit guarded adapter can call it. |
| Obligation outcome | A false literal call, an unconstrained call, and an exhausted search. | Distinct refuted/unknown/budget reasons; none compiles through implicit runtime fallback. |
| Mode separation | Remove `static` from a clause using a currently accepted runtime predicate. | Existing executable contract behavior remains; it supplies no verified static summary. |
| Incremental invalidation | Strengthen an imported requirement while keeping ordinary types unchanged. | Recheck callers and report newly unproved calls; no stale cache acceptance. |

Planned verification ownership:

- Syntax, formatting, and binder scoping: parser/AST/type tests, accepted and
  rejected source-surface fixtures, and formatter round trips.
- Entailment and path facts: focused `veln-sema` tests using the truth/order
  model; cover success, boundary, rejection, and restoration after scope exit.
- Observable compiler behavior: `examples/specification/check/` cases for
  direct/indirect/imported calls, human related notes, JSON outcomes, and holes.
- Runtime preservation and checked lowering: focused `run/` and `test/`
  toolchain cases for the compiler/backend contract, including existing runtime
  mode and once-only evaluation. These are not stdlib API tests.
- Future stdlib adoption: API behavior belongs in Veln companion tests under
  `crates/veln-stdlib/veln/`, following the
  [test placement policy](../reference/toolchain-test-harness.md#test-placement-policy).
- Incremental checking: language-service tests that change only an imported
  static contract and compare saved diagnostics with a fresh check.

Before Ready selection, P1 must define operation-count budgets for long Boolean
conditions, deep aliases, and repeated call summaries. Compare doubled input
sizes against a contract-free control, record retained facts and proof steps,
and require a declared non-exponential work bound or deterministic unknown
cutoff. Wall-clock samples are supporting evidence on a controlled runner,
not a portable acceptance threshold. Run generated/analysis workloads through
`bash scripts/agent-run` and the repository performance-audit workflow.

## Readiness and lifecycle

This proposal stays under Design In Progress until P1–P5 have precise grammar,
mandatory proof rules, trust boundaries, diagnostic schemas, work limits, and
reviewed feasibility evidence. In particular, callback contract joins, `?`
return facts, and invalid recursive-component publication must be exercised
before promising compositional guarantees. No independent ownership/effect or
numeric-language redesign is a prerequisite for the restricted slice.

A later Ready target must select a complete externally observable slice and
its acceptance cases. During implementation, promote checked examples and
current behavior into `contracts.md`, `types.md`, and the relevant grammar,
diagnostic, editor, and package-interface specifications. Update their
`update-when` dependents when the corresponding behavior changes. The current
specification and examples need no behavior edits for this documentation-only
proposal. Remove completed proposal sections and retire this page when all
remaining planned work is either implemented or explicitly closed.
