# Discussion Result: Contract Predicate Parsing

Status: accepted-proposal

## Picked Question

- Should contract clauses be parsed as full expressions and rejected by
  contract validation, or parsed through a narrower predicate production from
  the start?

## Decision

Parse `require`, `ensure`, and future `invariant` clauses through a narrow
contract predicate production from the start.

The contract predicate grammar should be a syntactic subset of ordinary Veln
expressions, sharing token spelling, operator precedence, names, field access,
literal forms, grouping, and pure call syntax where those forms are allowed by
the contract-expression decision. It should not initially accept syntactic
forms that contracts are guaranteed to reject, such as holes, `?` propagation,
pipeline chains, `match`, record or list construction, general allocation-like
forms, or effect-oriented runtime constructs.

This is a parser implementation rule, not a separate user-facing language. The
surface model remains: contracts are written with ordinary-looking boolean
predicates. The implementation model is: the parser produces contract predicate
nodes directly, then contract validation performs name resolution, type
checking, purity checking, effect checking, and static/runtime obligation
classification.

## Rationale

The earlier contract-expression result already decided that Veln contracts are
restricted pure boolean predicates rather than arbitrary executable programs.
Once that is true, accepting the whole expression grammar and rejecting many
forms later mostly increases recovery ambiguity. A missing `end`, an accidental
pipeline, or a `match` embedded in a contract should be diagnosed as a contract
predicate syntax error near the clause, not as a successfully parsed expression
that fails in a later semantic phase.

Design by Contract makes contract clauses part of the executable program
boundary, but Meyer does not require those clauses to be parsed as the same
language category as every program expression. The important property is that
preconditions, postconditions, and invariants are precise obligations. For
Veln, a dedicated predicate production preserves that precision while giving
the parser a smaller recovery surface.

JML and Dafny are useful engineering precedents. JML assertions are written in
a Java-like expression notation but live in specification contexts with
specification-only constructs and purity restrictions. Dafny similarly gives
specification contexts their own admissible expression space, separating
verification-facing predicates from effectful method bodies. Both designs
support the same lesson for Veln: the notation may look familiar, but the
context should constrain what can be parsed and then validated.

Liquid Types provides the static-analysis argument. Strong reasoning about
predicates is tractable because the predicate language is controlled and
solver-friendly, not because arbitrary program expressions are accepted and
interpreted after the fact. Veln's first slice is not yet a refinement-type
system, but choosing a bounded predicate grammar now keeps the later path to
stronger checking open.

For agents, the main benefit is repair locality. A parse diagnostic can say
that a contract expected a boolean predicate atom, comparison, conjunction, or
pure call, and it can recover at the next contract clause, body line, or `end`.
If the same text were first parsed as a broad expression, the diagnostic would
arrive later with less useful parser context and weaker synchronization
anchors.

## First-Slice Rule

- `ContractClause` should parse a dedicated `ContractPredicate` production, not
  the full `Expr` production.
- `ContractPredicate` shares ordinary expression spelling and precedence for
  literals, names, explicit result bindings in `ensure`, field access,
  grouping, primitive arithmetic, comparisons, boolean connectives, and pure
  function calls.
- The parser should reject contract-only syntax errors before semantic
  validation when a construct is outside the predicate grammar.
- Contract validation still owns name resolution, type checking, result-binding
  scope, purity, effect checking, and obligation status.
- A syntactically valid predicate may still be semantically invalid, for
  example because it calls an effectful function or returns a non-boolean type.
- Unsupported expression forms that the narrow parser can identify should
  produce `kind: "contract"` parse diagnostics with `details.phase: "parse"`
  and a contract-oriented `parser_context`.
- Unsupported forms discovered only after parsing, such as a call resolved to
  an effectful function, should produce `kind: "contract"` diagnostics with
  `details.phase: "contract"`.
- The grammar used by hole `satisfy` predicates should reuse the same
  `ContractPredicate` production, with the surrounding hole syntax adding the
  explicit candidate binding.

## Open Detail

The predicate grammar sketch in the first-slice grammar is the initial
implementation target. It is intentionally small: boolean operators,
comparisons, primitive arithmetic inside comparisons, grouping, names, field
access, literals, explicit result bindings where in scope, and plain or
qualified calls that validation can prove pure.

Quantifiers, collection predicates, pattern matching inside predicates, and
specification-only constructs such as `old(...)` remain deferred or separately
decided. When added, they should extend `ContractPredicate` directly rather
than entering through the full expression grammar by accident.

The formatter should preserve the ordinary expression look of predicates even
though the parser uses a separate production.

## References

- Meyer, B. (1997). *Object-Oriented Software Construction* (2nd ed.).
  Prentice Hall PTR. https://archive.eiffel.com/doc/oosc/
- Leavens, G. T., Cheon, Y., Clifton, C., Ruby, C., & Cok, D. (2013).
  *JML Reference Manual: Introduction*.
  https://www.cs.ucf.edu/~leavens/JML/jmlrefman/jmlrefman_1.html
- The dafny-lang community. (2026). *Dafny Reference Manual*.
  https://dafny.org/dafny/DafnyRef/DafnyRef
- Rondon, P. M., Kawaguchi, M., & Jhala, R. (2008). Liquid types.
  *PLDI 2008*, 159-169. https://doi.org/10.1145/1375581.1375602

## Consequence

The first parser and golden diagnostics get a smaller, contract-aware recovery
surface. Veln keeps contracts visually close to expressions while making the
checker pipeline clearer: parse only predicate-shaped syntax, then validate
semantic admissibility and classify each obligation.
