# Forbid Empty Effects Declarations

Status: proposed

This page proposes making omission the only source spelling for pure
declaration-level effects. It is proposal work, not current language behavior.

## Read First

- Current source declaration grammar:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current function, test, and function-type annotations:
  [../specification/types.md](../specification/types.md).
- Current effect labels and effect inference:
  [../specification/names-effects.md](../specification/names-effects.md).

## Current Boundary

The implemented grammar allows `effects []` anywhere a declaration-level
effect clause is accepted. Public functions and tests currently use
`effects []` to spell an explicitly pure boundary, while private functions may
omit the whole clause and rely on inference.

That creates two source spellings for pure private declarations and makes pure
public declarations more verbose than effectful ones need to be. It also makes
generated code choose between a redundant empty list and omission even though
both mean the same inferred effect set.

## Target

Pure `fn` declarations must omit the declaration-level `effects [...]` clause.
The source spelling `effects []` is invalid on a function declaration.

Non-empty effect clauses remain the explicit spelling for declarations that
perform effects:

```veln
pub fn print_line(text: String) -> () effects [stdio]
  stdio::println(text)
end
```

Pure public functions keep explicit parameter and return annotations, but their
empty declaration effect set is represented by the absence of an `effects`
clause:

```veln
pub fn double(value: Int) -> Int
  value * 2
end
```

Test declarations should follow the same declaration-level rule. A pure test
omits the effects clause, and an effectful test writes a non-empty clause.

## Semantics

- Absence of a declaration-level effect clause means the declared effect set is
  empty.
- A declaration-level `effects [...]` clause must contain at least one known
  effect label.
- A declaration with inferred effects that are not covered by its declaration
  reports the existing missing-effect diagnostic route.
- Public functions and tests that omit `effects [...]` are accepted only when
  their inferred effect set is empty.
- Private functions may still omit `effects [...]` and rely on inferred direct
  and transitive effects for callers.

## Diagnostics

The checker should report a targeted diagnostic when a declaration writes an
empty effect list. The primary message should state the failed source fact, for
example:

```text
empty effects list is not allowed on a function declaration
```

Related notes may suggest the repair:

- remove the clause when the inferred effect set is empty
- replace the empty list with the required non-empty labels when the body
  performs effects

Missing-effect diagnostics should stop suggesting `effects []` as the pure
repair. For an omitted public or test clause whose body performs effects, the
repair hint should name the required non-empty effect list.

## Formatter

`veln fmt` should preserve valid omission for pure declarations. It should not
introduce `effects []` as a canonical form.

The formatter does not need to silently repair invalid `effects []`. Static
checking should own the diagnostic and repair hint so users and agents see why
the clause is no longer accepted.

## Non-Goals

- Do not change the implemented effect labels.
- Do not require private effectful functions to declare non-empty effects.
- Do not change function type assignment compatibility.
- Do not decide a broader ban on empty effect lists in function type
  annotations unless a follow-up proposal explicitly expands the scope.
- Do not promote this proposal into `../specification/` until parser, checker,
  formatter, examples, and human output coverage agree on the new spelling.

## Acceptance Checks

- `fn helper() -> Int effects []` and
  `pub fn helper() -> Int effects []` produce a targeted empty-effects
  diagnostic.
- `pub fn helper() -> Int` is accepted when its inferred effect set is empty.
- A public function or test that omits `effects [...]` but reaches `stdio`,
  `fs`, `process`, or `concurrency` reports a missing-effect diagnostic with
  related provenance.
- Non-empty declarations such as `effects [stdio]` remain accepted when they
  cover the inferred effect set.
- Pure tests omit the effects clause, while effectful tests keep a non-empty
  clause.
- Human output coverage no longer tells users to write `effects []`.
- Examples and source-backed standard-library files stop using declaration
  `effects []` after the behavior is implemented.

## Update When

- Move the accepted behavior into `../specification/source-surface.md`,
  `../specification/types.md`, and `../specification/names-effects.md` only
  after implementation and coverage are complete.
- Update source-decision history only if the implementation changes the public
  effect boundary rationale rather than just the pure spelling.
