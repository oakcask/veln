---
role: specification
authority: normative
specification-coverage: usage=#usage; behavior=#declaration-behavior; limits=#limits-and-diagnostics
update-when: The SourceLocation prelude type or callsite function declaration and static-checking contract changes.
---

# Call-site Declarations

Call-site-aware declarations let a function body refer to compiler-supplied
source-location context. The current contract covers declaration syntax and
static checking. Runtime construction and propagation are not implemented.

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

## Limits and diagnostics

Parameters, result bindings, local bindings, and pattern bindings in a
modified function cannot be named `callsite`. The checker reports the rejected
binding at its name, identifies the modifier as the built-in origin, and
suggests renaming the binding. A rejected result binding does not replace the
built-in in an `ensure` clause or appear there with result-binding provenance.

An unresolved `callsite` value reference in an unmodified source function
reports that the modifier is missing and explains where to add it. Tests and
handler operation clauses cannot carry the modifier, so unresolved `callsite`
references in those bodies use the ordinary unresolved-name diagnostic. A
second modifier is rejected at the duplicate token and offers removal as a
repair.

The declaration contract does not supply hidden call data, construct a runtime
location, or propagate a location through direct or indirect calls. It does not
define generated-source mapping, backend metadata, or LSP and MCP presentation.
The checker accepts a call-site-aware declaration, but execution lowering
rejects a reachable reference to its built-in `callsite` value until runtime
location construction and the hidden call ABI are implemented. `veln run`
reports the lowering diagnostic and stops before backend execution.

## References

- Source grammar: [source-surface.md#executable-grammar](source-surface.md#executable-grammar).
- Executable grammar artifact: [source-surface-executable.pl](source-surface-executable.pl).
- Human diagnostic evidence:
  `examples/specification/check/callsite-declaration-contract/case.toml`.
- JSON diagnostic evidence:
  `examples/specification/check/callsite-declaration-contract-json/case.toml`.
- Formatter evidence:
  `examples/specification/fmt/callsite-modifier/case.toml`.
- Execution-boundary evidence:
  `examples/specification/run/callsite-runtime-boundary/case.toml`.
