---
role: specification
authority: normative
specification-coverage: usage=#usage; behavior=#declaration-behavior; limits=#limits-and-diagnostics
update-when: The SourceLocation value, callsite declaration, direct or indirect call propagation, related static and execution limits, or call-site semantic-token, completion, or signature-help contracts exposed through LSP or MCP change.
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

LSP semantic tokens identify the declaration modifier as a keyword and a bare
built-in body reference as a read-only variable. The built-in keeps that class
when a same-named function exists and when invalid source follows it with `(`.
A qualified leaf such as `other::callsite` remains an ordinary function token;
the built-in classification does not apply through a qualifier.
In an ordinary function, a parameter, result, local, or pattern binding named
`callsite` keeps its binding token class even when a source function has the
same name. Only a call target that resolves to that function receives the
function token class.
Completion offers the `callsite` modifier in an eligible source-function header
that does not already have it. For a header with a trailing comment, the
position immediately before the `#` marker remains eligible. A position inside
the comment, including the end of the line, does not offer the modifier.
Completion offers the built-in `callsite` local only in the body of a
call-site-aware source function; ordinary function bodies and other declaration
forms do not receive that candidate. In particular, runtime-contract clauses
and test bodies do not receive it. MCP exposes the same candidates through its
`completion` tool for saved workspace sources.

LSP signature help and the MCP `signature_help` tool render the complete source
declaration, including a trailing `callsite` modifier after any effects clause.
The modifier remains outside the parameter list, and active-parameter counting
uses only source parameters. A position inside the declaration's own header
does not produce signature help. An unfinished call in an earlier declaration
does not leak signature help into a later declaration header. Grouping
parentheses within a call argument do not hide the enclosing call's signature.
Signature help resolves bare ordinary source-function calls and qualified
source-function calls, including a qualified function whose name is `handle`.
The bare `handle (expression) with handler()` operator is not a function call
and does not produce signature help at its grouping parenthesis.
Inside a call-site-aware function body, the built-in `callsite` local is not
callable. A `callsite(` expression therefore does not fall back to a same-named
workspace or package function and does not produce signature help.
Every finite acyclic chain of public workspace function aliases resolves to its
target declaration before rendering; an alias cycle does not produce signature
help.

Presentation implementations apply an internal structural-work budget before
using the recursive parser on unfinished mutable source. The budget is a
safety mechanism, not a language, LSP, or MCP compatibility value, so clients
must not rely on an exact nesting or operator-count cutoff. Parse-derived
presentation can be omitted after the current budget is exceeded; parse-free
semantic-token classifications can still be returned. The
[MCP presentation contract](mcp.md#source-presentation) specifies and checks
the MCP result and preserved-session behavior. This page does not define a
cross-adapter availability guarantee for over-budget source.

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

When a call passes a call-site-aware value toward a runtime-backed collection,
option, result, or task callback consumer, the call expression becomes the
callback context. Ordinary forwarding calls preserve that bound value until
the runtime invokes the callback. The bound value also remains attached when
a task runs asynchronously. A call-site-aware wrapper forwards its existing
context instead of replacing it with an inner operation expression. If Veln
code invokes the value before it reaches the runtime, the ordinary indirect
call rules apply at that invocation. Callbacks without the modifier keep the
ordinary runtime callback ABI.

The built-in value behaves as an ordinary `SourceLocation` after it enters the
callee. The function can return it, nest it in another value, store it, or pass
it to an explicit `SourceLocation` parameter. The captured value remains
unchanged and observable after the originating function returns. Later
observation reads the retained value; it does not walk the current stack or
replace the value with the observation site. Its lines and columns are
one-based. Its offsets are zero-based UTF-8 byte offsets. End positions are
exclusive, and columns count Unicode scalar values.

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

Direct-call construction uses the call expression's complete source span. An
ordinary package-selected source therefore copies its package-relative path
into `file`. A generated source can identify one original source and associate
generated byte boundaries with original line, column, and byte-offset
coordinates. A mapping can list individual boundaries or declare a verbatim
copied region whose scalar boundaries retain their relative columns and byte
offsets. If both boundaries of the call expression map, `file` and all six
coordinates come from the original source. If either boundary has no mapping,
or the generated source has only an origin path, the complete generated or
virtual span is used instead. The value never combines a path from one source
with coordinates from another source.

The generated-source mapping does not interpolate between sparse mapping
entries, infer mappings from equal offsets, transform the contents of a copied
region, combine multiple original sources, or compose mappings from multiple
generation stages.

A library-generated virtual source derives its exposed path from a canonical
package-relative source path and a stable generator identity. The resulting
path has the form `<source>#<identity>`. The identity is one non-empty path
segment. Leading `./` segments are removed before construction. Construction
rejects an absolute source path, remaining `.` or `..` segments, empty
segments, an already-virtual source path, and an identity that contains a path
separator, `#`, or `:`. Equivalent logical sources and generator identities
therefore expose the same virtual `file` value when their source trees have
different absolute roots.

For a workspace source, `package` is empty and `module` is the caller's
logical module path. For a dependency source, `package` is the dependency's
public package identity and `module` is its logical package-local module path;
the internal `package::module` resolution spelling is not exposed. The `file`
field is package-relative. Two dependencies can therefore expose equal
`module` and `file` values for the same logical module and relative path while
remaining distinct through `package`. Loading equivalent project and
dependency trees from different absolute roots produces identical values for
all `SourceLocation` fields. The same relocation rule applies to the canonical
virtual paths of library-generated sources.

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
expression from which to obtain a location.
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
- Dependency collision execution evidence:
  [`callsite-dependency-source-identity`](../../examples/specification/run/callsite-dependency-source-identity/).
- Unicode coordinate evidence:
  the `callsite-unicode-coordinates` run specification case.
- Mapped doctest-origin evidence:
  [`callsite-generated-origin-runtime`](../../examples/specification/test/callsite-generated-origin-runtime/).
- Generated-source fallback evidence:
  `crates/veln-backend-jvm/src/tests/basic_backend.rs` covers mapped and
  fallback direct and indirect calls, `crates/veln-source/src/tests.rs` covers
  all-or-nothing boundary lookup, and
  `crates/veln-ast/src/tests/wire_round_trip.rs` preserves incomplete metadata
  without changing the fallback.
- Canonical virtual-source relocation evidence:
  `crates/veln-source/src/tests.rs`,
  `crates/veln-test/src/tests/doctest_source_locations.rs`, and
  `crates/veln-analysis/src/tests/source_location_identity.rs`.
- Deferred-observation lifetime evidence:
  [`callsite-deferred-observation`](../../examples/specification/run/callsite-deferred-observation/).
- Indirect-call propagation evidence:
  [`callsite-indirect-runtime`](../../examples/specification/run/callsite-indirect-runtime/).
- Runtime-backed collection and task callback evidence:
  [`callsite-runtime-callbacks`](../../examples/specification/run/callsite-runtime-callbacks/).
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
- LSP presentation contract:
  [Editor Support](editor-support.md#lsp-completion-and-signature-help).
- Checked LSP presentation evidence:
  [`callsite-presentation`](../../examples/specification/lsp/callsite-presentation/).
- MCP presentation contract:
  [MCP Server](mcp.md#source-presentation).
- Checked MCP presentation evidence:
  [`callsite-presentation`](../../examples/specification/mcp/callsite-presentation/).
