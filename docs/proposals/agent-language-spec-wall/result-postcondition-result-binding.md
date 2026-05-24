# Discussion Result: Postcondition Result Binding

Status: accepted-proposal

## Picked Question

- Can contract clauses safely reference `result`, or should postconditions use a
  more explicit binding syntax?

## Decision

Postconditions should use an explicit result binding instead of a magic bare
`result` name.

In the first slice, a function whose `ensure` clauses need to refer to the
returned value should bind that value in the return type position:

```veln
fn clamp(x: Int, low: Int, high: Int) -> value: Int
  require low <= high
  ensure low <= value
  ensure value <= high
end
```

The result binding is a contract-facing name. It is visible to `ensure` clauses
and related diagnostics for that function, but not to `require` clauses, not to
the function body, and not to callers. It is not a mutable out-parameter and
does not change the ordinary return mechanism.

Functions that do not need to mention the returned value may keep the shorter
return type form:

```veln
fn is_empty(items: List<Item>) -> Bool
  ensure true
end
```

## Rationale

Design by Contract makes the returned value a natural part of a postcondition:
the implementation promises a relation between the input state, the output
state, and the returned value. Mature contract languages demonstrate that this
reference must be special in some way. JML uses the escaped specification name
`\result` in `ensures` clauses, and its reference material calls out that the
backslash namespace avoids conflicts with Java identifiers. Dafny gives another
useful precedent: a function result can be referred to by a function call, or
the result can be given an explicit name in the signature and used in the
postcondition.

For Veln, a bare `result` identifier is too easy to confuse with an ordinary
name and too close to the built-in `Result` type family. Reserving it globally
would also spend a common word for a feature that only matters inside
postconditions. JML avoids this by using a separate escaped namespace, but Veln
does not otherwise need such a namespace in the first slice.

An explicit result binding keeps the contract local and readable. It lets the
author choose a semantic name such as `value`, `items`, `parsed`, or `total`,
which gives agents and reviewers better repair context than a universal
placeholder. If the contract fails, diagnostics can point to the named result
binding and the failing `ensure` expression without teaching a hidden magic
identifier rule.

The cost is small because public functions already require explicit return
types. A named result is extra syntax only when a postcondition needs the
returned value. Private helpers can also use it when contracts are present, but
they are not forced to name every result.

This syntax also leaves room for a refinement-type interpretation without
forcing refinement syntax into the first slice. A signature such as
`-> value: Int` plus `ensure low <= value` can be checked as a contract-facing
binding today, while a later checker may internally treat the return position
as an `Int` refined by the `ensure` predicates that mention `value`. That model
supports compile-time discharge when the refinement is provable and runtime
checking when the predicate is valid but not statically settled.

## First-Slice Rule

- `ensure` clauses may reference a function's returned value only through an
  explicit result binding.
- The working syntax is `-> name: Type` for a single returned value.
- `-> Type` remains valid when no postcondition needs the returned value.
- The result binding is in scope for `ensure` clauses and contract diagnostics
  only.
- The result binding is not in scope for `require` clauses because
  preconditions describe the caller's entry obligation before the value exists.
- The result binding is not in scope in the function body and is not assignable.
- The result binding may be used by the checker as the value variable when
  lowering postconditions to internal return refinements or Hoare-style
  computation specifications.
- Bare `result` has no special meaning. If an `ensure` clause references
  `result` without a binding named `result`, the checker should report an
  ordinary unresolved-name diagnostic with a repair hint to add an explicit
  result binding.
- If the syntax `-> name: Type` conflicts with the final grammar, the grammar
  may switch to an equivalent explicit form such as `-> Type as name`; the
  observable rule is that the returned value is named explicitly, not via a
  hidden reserved identifier.

## Open Detail

Multiple return values are outside the first slice. If they are added later,
each returned component should have its own explicit binding rather than
reviving a single implicit `result` name.

The first slice does not decide pattern matching inside contracts over
`Result<T, E>` or `Option<T>`. A function returning `Result<T, E>` binds the
outer returned value. Later contract syntax may add safe destructuring or
predicate helpers for success and failure cases.

The exact diagnostic code names remain open. The important distinction is that
using an unbound result name is a local contract name-resolution error, while a
bound result with the wrong type is an ordinary contract type error.

## References

- Meyer, B. (1997). *Object-Oriented Software Construction* (2nd ed.).
  Prentice Hall PTR. https://archive.eiffel.com/doc/oosc/
- Leavens, G. T., Cheon, Y., Clifton, C., Ruby, C., & Cok, D. (2013).
  *JML Reference Manual: Introduction*.
  https://www.cs.ucf.edu/~leavens/JML/jmlrefman/jmlrefman_1.html
- The dafny-lang community. (2026). *Dafny Reference Manual*.
  https://dafny.org/dafny/DafnyRef/DafnyRef
- Freeman, T. S., & Pfenning, F. (1991). Refinement types for ML.
  *PLDI 1991*, 268-277. https://doi.org/10.1145/113446.113468
- Xi, H., & Pfenning, F. (1999). Dependent types in practical programming.
  *POPL 1999*, 214-227. https://doi.org/10.1145/292540.292560
- Nanevski, A., Morrisett, G., & Birkedal, L. (2006). Polymorphism and
  separation in Hoare type theory. *ICFP 2006*, 62-73.
  https://doi.org/10.1145/1160074.1159812

## Consequence

Veln avoids a magic postcondition identifier while still making returned values
easy to specify. Agents get a stable, named repair anchor in public contracts,
and the grammar keeps room for future richer postcondition syntax without
committing to a global `result` keyword.
