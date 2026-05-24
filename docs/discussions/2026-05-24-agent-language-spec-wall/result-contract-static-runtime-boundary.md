# Discussion Result: Contract Static Runtime Boundary

Date: 2026-05-24

## Picked Question

- How much of a contract should be available to static analysis versus runtime
  checking only?

## Decision

Every first-slice contract clause should be statically available for syntax,
name resolution, type checking, purity checking, effect checking, diagnostic
rendering, hole context, and documentation. Only a conservative subset should
be statically discharged as proven or disproven.

The first implementation should treat runtime checking as the semantic default
for executable `require` and `ensure` clauses whose truth cannot be proven from
local facts. Static analysis may prove simple obligations, reject impossible or
ill-formed clauses, and report obvious contradictions, but it should not require
a general verifier before contracts become useful.

Put another way: compile-time checking is the first attempt, not the only
attempt. A valid contract obligation should be classified during `veln check`;
if the checker can prove or refute it from supported facts, the obligation is
settled at compile time. If the checker cannot settle it but the predicate is
valid and executable, the obligation remains part of the function boundary and
is enforced at runtime by `veln run` and `veln test`.

## Rationale

Design by Contract is useful for Veln because contracts make caller and
implementation obligations explicit at the function boundary. Meyer's original
framing assumes executable assertions that can be checked at runtime, which
fits Veln's goal of short repair loops better than waiting for a complete
verification system.

Runtime contracts also give a principled place for blame. Findler and
Felleisen's higher-order contract work shows that dynamic contract checking can
carry blame information even when the property is not statically known. Veln
already treats blame as repair-routing metadata, so an unknown-at-check-time
contract should usually become a runtime boundary check rather than a vague
static warning.

Full static contract reasoning is valuable, but the first slice should avoid
making it the baseline. Liquid Types show that strong static checking becomes
tractable when predicates are restricted and solver-facing refinements are
carefully controlled. Dafny shows the power of verification-oriented
specifications, while JML shows a mixed ecosystem where specification clauses
can support both runtime assertion checking and static tools. Those systems
argue for a layered design: validate and expose contracts statically now, then
add stronger proof machinery around a disciplined subset later.

For agents, the important distinction is not "static or dynamic" but "what can
the checker say with confidence?" A static contract diagnostic should separate
format/type/purity errors, definite failures, proven obligations, and obligations
that require runtime evidence. This keeps `veln check --json` actionable without
pretending that every pure boolean predicate is solver-complete.

Refinement and dependent type systems give a useful implementation model for
postconditions: a result contract can be viewed internally as a return type
refinement, for example "an `Int` value satisfying these predicates over the
arguments and returned value." Hoare Type Theory makes this more explicit by
putting preconditions and postconditions into the computation type itself. Veln
does not need to expose that full type-theoretic form in the first slice, but
the checker should be designed so `ensure` clauses can later elaborate toward
return refinements or Hoare-style computation specifications without changing
the user-facing contract classification.

## First-Slice Rule

- `veln check` validates all contract clauses for parse errors, names, types,
  purity, allowed expression forms, and declared effects.
- Contract validation is static even when the contract's truth is not statically
  decidable.
- Every valid contract obligation receives a check-time classification before
  any runtime behavior is considered.
- Static discharge is conservative. The checker may mark an obligation proven
  or disproven only when it follows from local, implementation-supported facts
  such as literal values, primitive comparisons, type refinements, simple boolean
  simplification, and directly visible `require` assumptions.
- Proven obligations need no runtime check for ordinary `run` and `test`
  execution because the checker has already discharged them under the current
  compiler rules.
- Disproven obligations are check-time contract failures rather than deferred
  runtime failures.
- Unknown obligations are not static errors. They are reported as
  runtime-required obligations when the clause is otherwise valid and executable.
- `veln run` and `veln test` should enforce valid runtime-required `require`
  and `ensure` clauses at function boundaries in the first implementation.
- A contract that uses rejected syntax, effectful computation, non-total
  operations without guards, or unsupported specification forms is a contract
  validation error rather than a runtime-only contract.
- JSON diagnostics for contract obligations should distinguish validation
  errors from semantic failures and should expose an obligation status such as
  `proven`, `disproven`, `runtime_required`, or `unknown_unsupported`.
- `runtime_required` means "not statically discharged by this checker," not
  "ignored by `check`."
- Hole and repair diagnostics may use valid contract predicates as context even
  when those predicates are not statically proven.

## Open Detail

The exact static-discharge engine is not decided here. The first implementation
can start with syntactic simplification and local environment facts, then later
add refinement inference or SMT-backed checks behind the same obligation-status
model.

The exact internal representation of postconditions is also open. It may be a
separate contract table at first, but it should preserve enough information to
support a future lowering to return refinements or Hoare-style computation
types.

The runtime policy for optimized or release builds remains open. This decision
requires `run` and `test` to enforce runtime-required contracts early; it does
not decide whether a future deployment mode may disable selected checks.

Quantifiers, collection-wide predicates, and user-defined logical lemmas remain
deferred. When introduced, they should enter through the same classification:
statically valid, possibly statically discharged, otherwise runtime-required
only if executable and bounded.

## References

- Meyer, B. (1997). *Object-Oriented Software Construction* (2nd ed.).
  Prentice Hall PTR. https://archive.eiffel.com/doc/oosc/
- Findler, R. B., & Felleisen, M. (2002). Contracts for higher-order
  functions. *ICFP 2002*, 48-59.
  https://dblp.org/rec/conf/icfp/FindlerF02
- Rondon, P. M., Kawaguchi, M., & Jhala, R. (2008). Liquid types.
  *PLDI 2008*, 159-169. https://doi.org/10.1145/1375581.1375602
- Freeman, T. S., & Pfenning, F. (1991). Refinement types for ML.
  *PLDI 1991*, 268-277. https://doi.org/10.1145/113446.113468
- Xi, H., & Pfenning, F. (1999). Dependent types in practical programming.
  *POPL 1999*, 214-227. https://doi.org/10.1145/292540.292560
- Nanevski, A., Morrisett, G., & Birkedal, L. (2006). Polymorphism and
  separation in Hoare type theory. *ICFP 2006*, 62-73.
  https://doi.org/10.1145/1160074.1159812
- Swamy, N., Hritcu, C., Keller, C., Rastogi, A., Delignat-Lavaud, A.,
  Forest, S., Bhargavan, K., Fournet, C., Strub, P.-Y., Kohlweiss, M.,
  Zinzindohoue, J. K., & Zanella-Beguelin, S. (2016). Dependent types and
  multi-monadic effects in F*. *POPL 2016*, 256-270.
  https://doi.org/10.1145/2837614.2837655
- Greenberg, M., Pierce, B. C., & Weirich, S. (2012). Contracts made
  manifest. *Journal of Functional Programming*, 22(3), 225-274.
  https://doi.org/10.1017/S0956796812000135
- Leavens, G. T., Cheon, Y., Clifton, C., Ruby, C., & Cok, D. (2013).
  *JML Reference Manual: Introduction*.
  https://www.cs.ucf.edu/~leavens/JML/jmlrefman/jmlrefman_1.html
- The dafny-lang community. (2026). *Dafny Reference Manual*.
  https://dafny.org/dafny/DafnyRef/DafnyRef

## Consequence

Contracts become useful in the first implementation without committing Veln to
a full verifier. Agents get stable static diagnostics for contract shape and
clear runtime obligations for semantic checks, while the language keeps room
for later refinement-style and verifier-backed analysis.
