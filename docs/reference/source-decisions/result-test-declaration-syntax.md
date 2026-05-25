# Discussion Result: Test Declaration Syntax

Status: implemented

## Picked Question

- How should source code mark executable test cases so `veln test` does not
  have to guess which zero-argument functions are tests?

## Decision

Use a dedicated top-level `test` declaration for user-authored test cases.
Ordinary `fn` declarations are never test cases by syntax alone.

The first source spelling is:

```veln
use stdio

test prints_message() -> () effects [stdio]
  stdio::println("sample test ran")
  ()
end

test returns_ok() -> Result((), String) effects []
  Ok(())
end
```

`test` is a declaration keyword, not a visibility modifier. `pub test` is not
valid. Test declarations are runner entries and executable specifications, not
public API functions.

## Grammar Update

This result updates the first-slice grammar by adding `TestDecl` as a top-level
item. Function type syntax still uses `fn`; `test` is only a declaration head.

The implemented production is maintained in
[Source Surface](../language/source-surface.md#grammar).

The first slice requires the empty parameter list, explicit return type, and
explicit `effects [...]` clause on every `test` declaration.

## First-Slice Rules

- `veln test` selects `test` declarations. Executable doctest examples remain
  outside the implemented source surface. The runner must not select ordinary
  `fn` declarations merely because they have zero parameters.
- A `test` declaration must have no parameters. A non-empty parameter list is a
  test-shape diagnostic at the parameter span.
- A `test` declaration must return `()` or `Result((), E)`. Completing a `()`
  test passes. Returning `Ok(())` passes. Returning `Err(error)` fails the case.
- A `test` declaration must include `effects [...]`, including `effects []` for
  pure tests. Tests are not public API, but they are externally selected tool
  entries, so their effect boundary should be explicit.
- Test declarations are not ordinary callable functions. Their bodies can call
  ordinary functions and use imports from the same project context, but user
  code cannot call a test by name.
- Test names live in the same declaration namespace as ordinary functions.
  `test foo` and `fn foo` cannot coexist in one checked source set, and
  duplicate function-like names are rejected with `name.duplicate`.
- `*_test.veln` remains a useful organization convention and discovery hint,
  but it does not change the meaning of `fn`.
- Explicit file and directory targets bound the source set exactly as before;
  within that source set, `veln test` selects only `test` declarations.
- `veln check` parses and checks `test` declarations with the rest of the
  source set so stale tests do not hide from ordinary static diagnostics.
- `veln fmt` preserves the `test` declaration head and formats the signature
  and body like an `fn` declaration.

## JSON And Diagnostics

`veln test --json` keeps `case.kind: "test"` for source `test` declarations.
The `source.node_id` prefix should distinguish test declarations from ordinary
functions, for example `test-3` instead of `fn-3`.

Shape diagnostics should keep the primary message focused on the failed test
fact at the declaration span:

- `test.parameters`: `test declaration has parameters`
- `test.return_type`: `test declaration returns <Type>`
- `name.duplicate`: `duplicate function declaration name <Name>`
- `effect.missing_test`: `test declaration has no effects annotation`

Related notes point to the accepted test shape or effect declaration syntax.

## Compatibility

The older bootstrap behavior that treated zero-argument functions in selected
test files as cases is no longer part of the implemented language reference.

## Rationale

Selecting tests by zero-argument function shape is cheap to implement but weak
as a language rule. It makes helpers, examples, and intended cases look the
same until the runner applies out-of-band file and arity conventions. That is
especially poor for agents: the source span does not say whether a function is
an executable specification, a helper, or an accidentally runnable entry.

A declaration keyword makes the test role local, parseable, and stable in the
AST. It also gives diagnostics a precise place to attach test-only constraints
such as zero parameters, allowed return types, and explicit effects. Keeping
`test` separate from `pub fn` avoids mixing public API boundary rules with
tool-entry rules while still making tests visible to `check`, `fmt`, and
structured JSON.

The syntax remains close to `fn` on purpose. Tests are executable functions
with a special selection role, not a separate assertion language. Reusing the
same signature, contracts, body, effect declaration, and return semantics keeps
the parser and formatter small while removing the ambiguous part: discovery.

## Consequence

The durable test source syntax becomes self-describing:

```veln
test calculates_total() -> Result((), String) effects []
  Ok(())
end
```

The implementation adds `test` as a lexer keyword and AST item, teaches
semantic analysis the test-shape rules, and updates discovery to ignore
ordinary zero-argument `fn` declarations.
