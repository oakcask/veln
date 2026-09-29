---
role: proposal
update-when: Call-site-aware functions, source-location values, generated-source mapping, or call-site lowering is implemented or redesigned.
---

# Call-site Source Location

Libraries that produce diagnostics, logs, events, and traces need the user's
call site rather than the source location inside a wrapper function. Veln must
provide this without making each observability function a compiler-recognized
special case.

The location must be captured when the library API is called. A trace can be
finished or exported after the originating call stack no longer exists, so a
later stack walk cannot recover the required logical call site.

## Outcome

Add the standard `SourceLocation` value and a `callsite` function modifier that
introduces a built-in `callsite` local variable.

```veln
pub type SourceLocation = {
  package: String,
  module: String,
  file: String,
  start_line: Int,
  start_column: Int,
  start_offset: Int,
  end_line: Int,
  end_column: Int,
  end_offset: Int,
}
```

Lines and columns are one-based. Columns count Unicode scalar values. Offsets
count UTF-8 bytes. `file` is a package-relative or virtual source path; it is
never a machine-specific absolute path. `package` and `module` disambiguate
equal relative paths from different dependencies.

## Call-site-aware Functions

A function can place the `callsite` modifier at the end of its header:

```veln
pub fn info(
  message: String,
  attributes: Attributes,
) -> () effects [Observe] callsite
  observe::emit(LogInfo(message, attributes, callsite))
end
```

The proposed grammar addition is:

```ebnf
CallsiteModifier ::= "callsite"
```

`CallsiteModifier` follows the optional effects clause in a function header.
The modifier introduces a built-in local variable named `callsite` with type
`SourceLocation`. The modifier and the local variable use the same contextual
keyword because the modifier's purpose is to introduce that value.

Within a call-site-aware function, a parameter, result binding, local binding,
or pattern binding cannot use the name `callsite`. Outside a call-site-aware
function, `callsite` remains an ordinary identifier. An unresolved `callsite`
reference in a function body reports that the `callsite` modifier introduces
the built-in local variable.

## Propagation

A call to a call-site-aware function supplies hidden call-site context. The
supplied value depends on the calling function:

| Calling function | Supplied value |
| --- | --- |
| Not call-site-aware | The location of the call expression. |
| Call-site-aware | The calling function's built-in `callsite` value. |

The second rule makes a call-site-aware wrapper transparent by default:

```veln
pub fn warning(message: String) -> () effects [Observe] callsite
  info(message, {})
end
```

The final callee observes the location at which the user called `warning`.
Direct and indirect calls follow the same propagation table. Devirtualization
and inlining do not change the observed location.

The built-in local variable is an ordinary `SourceLocation` value. A function
can pass it to another function, store it in an event or trace, or return it.
The implicit context cannot be overridden at a call expression. A library that
accepts an already available location can use an ordinary `SourceLocation`
parameter. Such a function is not call-site-aware unless its header also has
the `callsite` modifier.

The `callsite` modifier is visible in source, documentation, completion, and
signature help. Completion inside the function body includes the built-in
local variable. The modifier does not contribute to ordinary callable arity or
function type.

The call ABI must therefore carry a hidden source location for direct and
indirect calls. A function without the modifier does not expose or use that
hidden value. Function interface metadata records whether a function is
call-site-aware so separate compilation does not infer the calling convention
from its body. An implementation can specialize direct calls, but the
observable location remains defined by the propagation table.

## Generated and Virtual Sources

When the compiler has an origin mapping, `SourceLocation` identifies the
mapped user source. Otherwise it identifies the generated or virtual source.
The file field uses the same canonical virtual-source naming contract as
diagnostics. Moving a package to another machine must not change the exposed
file value.

## Acceptance Model

| Case | Source form | Required observation | Planned evidence |
| --- | --- | --- | --- |
| S1 | A non-call-site-aware function directly calls a call-site-aware function. | The callee's `callsite` value identifies the call expression. | Run specification case. |
| S2 | A call-site-aware wrapper calls another call-site-aware function. | The final callee observes the outer user's site. | Nested-wrapper run case. |
| S3 | A call-site-aware function passes `callsite` to an ordinary `SourceLocation` parameter. | The ordinary parameter receives the same value. | Type-check and run cases. |
| S4 | A call-site-aware function is invoked through a function value from a non-call-site-aware function. | The callee observes the indirect call expression and callable type checking remains unchanged. | Type-check and run cases. |
| S5 | The modifier is missing when the built-in variable is referenced, the modifier is duplicated, or a binding shadows the built-in variable. | Checking reports the failed declaration rule and a repair. | Check and check-JSON cases. |
| S6 | Source is generated and has an origin mapping. | The exposed location is the mapped user location. | Generated-source fixture. |
| S7 | A package is checked from two different absolute roots. | Exposed package, module, and file values are identical and contain neither root. | Relocation test. |
| S8 | Formatter, docs, LSP, and MCP present the declaration. | Each surface identifies the modifier and built-in local variable without changing ordinary arity. | Formatter, documentation, LSP, and MCP cases. |
| S9 | A trace retains a `callsite` value after its originating function returns. | Later observation reports the captured location without walking the current stack. | Deferred-observation run case. |

## Verification and Promotion

Implementation must extend executable grammar, accepted and rejected source
fixtures, AST wire encoding, semantic analysis, callable lowering, backend
metadata, formatting, documentation, LSP, and MCP. The run harness must compare
locations against a checked source fixture rather than machine paths.

After implementation, the current source-surface and name/effect
specifications must explain the `callsite` modifier and built-in local variable.
The source and execution specifications must explain `SourceLocation`, call-site
propagation, and generated-source mapping.

## Non-goals

- This proposal does not expose a runtime stack trace.
- Source locations are not stable identifiers across source edits.
- The proposal does not add general optional or default parameters.
- The proposal does not add syntax that overrides implicit call-site context at
  an individual call expression.
- The proposal does not expose an arbitrary current expression's location as a
  `SourceLocation` value.
- The proposal does not give libraries access to machine-specific source
  paths.
