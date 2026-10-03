---
role: specification
authority: normative
specification-coverage: usage=#usage; behavior=#declaration-behavior; limits=#limits-and-diagnostics
update-when: The SourceLocation value, callsite declaration, direct or indirect call propagation, or related static and execution limits change.
---

# Call-site Declarations

Call-site-aware declarations let a function body refer to compiler-supplied
source-location context. Direct and indirect calls construct and propagate
that context as an ordinary `SourceLocation` value.

## Usage

`SourceLocation` is exported by the standard prelude and is available without
an import. A source `fn` declaration places `callsite` after its optional
`effects` clause:

```veln
pub fn source_line() -> Int effects [] callsite
	callsite.start_line
end
```

The modifier introduces a built-in local named `callsite` with type
`SourceLocation`. The type is the structural record `{ package: String, module:
String, file: String, start_line: Int, start_column: Int, start_offset: Int,
end_line: Int, end_column: Int, end_offset: Int }`.

An ordinary function directly calls a call-site-aware function in the same way
as any other function. The compiler supplies the hidden context; source code
does not add an argument:

```veln
pub fn location() -> SourceLocation callsite
	callsite
end

pub fn caller() -> SourceLocation
	location()
end
```

## Declaration behavior

A source function accepts at most one `callsite` modifier. The modifier does
not add a parameter to the ordinary callable type and does not change callable
arity. A returned function type keeps its own `effects` clause when that clause
appears immediately before the modifier.

Inside the modified function, name resolution treats the built-in `callsite`
as a `SourceLocation` local. Outside a modified function, `callsite` remains an
ordinary identifier. The formatter preserves the modifier after the optional
function effects clause and formats the built-in reference like any other
local reference.

For a direct call from an ordinary function, the supplied value covers the
complete call expression from its callee through its closing parenthesis. For
a pipeline call, it covers the right-hand call expression and excludes the
piped argument and pipeline operator. For a direct call from a call-site-aware
function, the supplied value is that function's existing `callsite` value. A
chain of call-site-aware wrappers therefore preserves the outer ordinary
caller's call expression.

A call-site-aware declaration can be obtained and stored as an ordinary
function value. When an ordinary function invokes that value, the supplied
location covers the complete indirect call expression. The function-value
acquisition expression does not affect the location. When a call-site-aware
function invokes a function value, the invocation forwards that function's
existing `callsite` value. The hidden context remains outside the callable's
source parameter list, so indirect-call arity and arity diagnostics use only
the declared source parameters. Function values for declarations without the
modifier keep the ordinary calling convention and do not receive or use the
hidden value.

The built-in value behaves as an ordinary `SourceLocation` after it enters the
callee. The function can return it or pass it to an explicit
`SourceLocation` parameter. Its lines and columns are one-based. Its offsets
are zero-based UTF-8 byte offsets. End positions are exclusive, and columns
count Unicode scalar values.

Runtime-required `require`, `invariant`, and `ensure` predicates in a
call-site-aware function read the same supplied `SourceLocation` as the
function body. Their coordinates therefore identify the call expression
selected by the direct-call and wrapper propagation rules above. When such a
predicate directly calls another call-site-aware function, the predicate
callee receives that same value as hidden context. The source-level call keeps
its declared arity. Calling through a public function alias reaches the
aliased declaration with the same hidden context. Fixed and variadic source
arguments bind as they do for an ordinary direct call; the hidden context does
not enter the variadic argument sequence.

Direct-call construction copies the call expression's existing source
identifier into `file`. An ordinary package-selected source therefore uses its
package-relative path. The current `package` field is empty. The `module` field
is the caller's resolved module name, or empty when the caller has no resolved
module name. Generated-source origin mapping, canonical virtual-source naming,
dependency disambiguation, and relocation guarantees are not part of the
current value construction.

## Limits and diagnostics

Parameters, result bindings, local bindings, pattern bindings, and hole
`satisfy` candidates in a modified function cannot be named `callsite`. The
checker reports the rejected binding or candidate at its name, identifies the
modifier as the built-in origin, and suggests renaming it. A rejected result
binding does not replace the built-in in an `ensure` clause or appear there
with result-binding provenance. A rejected `satisfy` candidate does not replace
the built-in in its predicate.

An unresolved `callsite` value reference in an unmodified source function
reports that the modifier is missing and explains where to add it. Tests and
handler operation clauses cannot carry the modifier, so unresolved `callsite`
references in those bodies use the ordinary unresolved-name diagnostic. A
second modifier is rejected at the duplicate token and offers removal as a
repair.

A `veln run` entry cannot carry the modifier because it has no Veln call
expression from which to obtain a location. Generated-source origin mapping,
canonical virtual-source naming, dependency source-identity collisions,
relocation guarantees, deferred-observation lifetime guarantees, and
call-site-specific LSP and MCP presentation are not implemented.
Runtime-required contract predicates in ordinary functions do not construct
call-site context. Execution rejects a direct call from such a predicate to a
call-site-aware function because the enclosing function has no hidden context
to forward. This is a deliberate boundary rather than pending direct-call
propagation.

An unmodified function can use an ordinary binding named `callsite`, including
in its contracts, and execution treats that binding like any other local value.
Functions without the modifier retain their ordinary call ABI.

## References

- Source grammar: [source-surface.md#executable-grammar](source-surface.md#executable-grammar).
- Executable grammar artifact: [source-surface-executable.pl](source-surface-executable.pl).
- Human diagnostic evidence:
  `examples/specification/check/callsite-declaration-contract/case.toml`.
- JSON diagnostic evidence:
  `examples/specification/check/callsite-declaration-contract-json/case.toml`.
- Formatter evidence:
  `examples/specification/fmt/callsite-modifier/case.toml`.
- Direct-call and wrapper execution evidence:
  the `callsite-direct-runtime` run specification case.
- Unicode coordinate evidence:
  the `callsite-unicode-coordinates` run specification case.
- Indirect-call propagation evidence:
  [`callsite-indirect-runtime`](../../examples/specification/run/callsite-indirect-runtime/).
- Run-entry boundary evidence:
  the `callsite-entry-runtime-boundary` run specification case.
- Runtime contract built-in reference evidence:
  the `callsite-contract-runtime` run specification case.
- Runtime contract call propagation and failure evidence:
  the `callsite-contract-call-runtime` and `callsite-contract-failure` run
  specification cases.
- Ordinary-function runtime contract boundary evidence:
  [`callsite-contract-runtime-boundary`](../../examples/specification/run/callsite-contract-runtime-boundary/).
- Ordinary-identifier execution evidence:
  `examples/specification/run/callsite-ordinary-identifier/case.toml`.
