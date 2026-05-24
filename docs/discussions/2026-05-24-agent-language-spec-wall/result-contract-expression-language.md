# Discussion Result: Contract Expression Language

Date: 2026-05-24

## Picked Question

- Should `require`, `ensure`, and `invariant` be executable expressions in the
  core language, or a restricted contract language?

## Decision

Use a restricted contract expression language in the first slice.

Contract clauses should parse as Veln expressions only where those expressions
are pure, total enough for checking, and usable as boolean predicates. In the
first slice, `require` and `ensure` may use literals, names in scope, field and
record access, equality and ordering, boolean connectives, arithmetic on
primitive numeric values, pure built-in predicates, pure user functions that are
explicitly marked or inferred as pure, and `old(...)` or an equivalent
postcondition snapshot form if post-state comparison is needed.

Contract clauses must not call effectful functions, allocate, perform I/O,
mutate state, use non-determinism, inspect time or randomness, propagate
`Result` with `?`, or depend on holes that would make the clause uncheckable.
Unsupported expression forms should produce `kind: "contract"` diagnostics
that name the disallowed construct and, when possible, suggest moving the logic
into a pure helper.

## Rationale

Design by Contract treats preconditions, postconditions, and invariants as
program-facing obligations rather than prose. Meyer's formulation is useful for
Veln because it makes caller and implementation responsibilities explicit, and
it matches the existing contract blame decision. But using arbitrary executable
program expressions as contract bodies would make the first slice harder to
parse, evaluate, statically inspect, and explain.

The contract literature also points away from "anything executable" as the
first default. Findler and Felleisen show that executable higher-order
contracts can be given principled runtime behavior and blame, but they also
motivate the difficulty of treating arbitrary predicates over functions as
statically meaningful. Rondon, Kawaguchi, and Jhala's Liquid Types work points
in the opposite direction: strong static checking becomes tractable by
restricting predicates to solver-friendly refinements and qualifiers.

Mature specification languages make the same engineering split. JML lets
assertions mention pure methods and specification-only constructs such as
`\result`, while excluding arbitrary side-effecting Java behavior from
assertions. Dafny similarly separates specification clauses and specification
or ghost expressions from general effectful program actions; method calls are
not general specification expressions even when the parser shape is similar.

For Veln's agent-oriented goal, this restriction keeps contract diagnostics
short and repairable. If a clause fails to typecheck or uses an effectful call,
`veln check --json` can report a local contract-language error instead of
forcing the agent to reason about runtime ordering, hidden state changes, or
tool-specific verifier limits.

## First-Slice Rule

- `require` and `ensure` clauses accept only boolean contract expressions.
- Contract expressions are a checked subset of Veln expressions, not a
  separate surface syntax.
- The subset includes pure value operations, boolean connectives, comparisons,
  field or record access, primitive arithmetic, and calls to pure functions.
- Effectful calls, mutation, allocation, I/O, time, randomness, process access,
  `?` propagation, and general runtime-only constructs are rejected in
  contracts.
- Public functions that are used from contracts must either declare or infer no
  effects under the first-slice effect rules.
- Contract diagnostics should distinguish normal type errors from
  contract-language rejections, so repair tools can tell whether to change the
  predicate, annotate purity, or move logic into a pure helper.
- `invariant` uses the same expression subset once invariants are introduced,
  but this decision does not decide invariant attachment points or blame.

## Open Detail

The exact grammar production is now resolved by
[Contract Predicate Parsing](result-contract-predicate-parsing.md): contract
clauses use a narrower predicate production from the start. The observable rule
remains that unsupported forms produce structured contract diagnostics.

Quantifiers, collection comprehensions, and user-defined logical operators are
deferred. They are important for stronger specifications, but the first slice
should prove the repair loop with a small, predictable predicate language
before adding solver-facing constructs.

The syntax for post-state values remains open. JML uses `\result`, and Dafny
uses named returns plus `old(...)`-style state references. Veln still needs to
decide whether postconditions refer to `result`, named return values, or an
explicit binding form.

## References

- Meyer, B. (1997). *Object-Oriented Software Construction* (2nd ed.).
  Prentice Hall PTR. https://archive.eiffel.com/doc/oosc/
- Findler, R. B., & Felleisen, M. (2002). Contracts for higher-order
  functions. *ICFP 2002*, 48-59.
  https://dblp.org/rec/conf/icfp/FindlerF02
- Rondon, P. M., Kawaguchi, M., & Jhala, R. (2008). Liquid types.
  *PLDI 2008*, 159-169.
  https://doi.org/10.1145/1375581.1375602
- Leavens, G. T., Cheon, Y., Clifton, C., Ruby, C., & Cok, D. (2013).
  *JML Reference Manual: Introduction*.
  https://www.cs.ucf.edu/~leavens/JML/jmlrefman/jmlrefman_1.html
- The dafny-lang community. (2026). *Dafny Reference Manual*.
  https://dafny.org/dafny/DafnyRef/DafnyRef

## Consequence

Contracts remain executable and close to code, but they are not arbitrary Veln
programs. The first checker can give stable, local diagnostics for contract
syntax, type, purity, and effect violations, while leaving stronger
specification constructs for later slices.
