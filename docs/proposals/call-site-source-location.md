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

The declaration and static-checking foundation is current behavior specified in
[Call-site Declarations](../specification/call-site-declarations.md).
This proposal tracks only the runtime propagation and presentation work below.

Runtime-constructed `SourceLocation` values use one-based lines and columns.
Columns count Unicode scalar values. Offsets count UTF-8 bytes.

## Remaining call-site-aware function behavior

The declaration form is implemented. Runtime calls still need to supply the
location represented by that declaration:

```veln
pub fn info(
  message: String,
  attributes: Attributes,
) -> () effects [Observe] callsite
  observe::emit(LogInfo(message, attributes, callsite))
end
```

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

Once runtime propagation supplies the built-in value, a function must be able
to pass it to another function, store it in an event or trace, or return it.
The implicit context cannot be overridden at a call expression. A library that
accepts an already available location can use an ordinary `SourceLocation`
parameter; runtime observation must distinguish that explicit argument from
the hidden context.

Completion and signature help must present the `callsite` modifier. Completion
inside the function body must include the built-in local variable.

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
| S3 | A call-site-aware function passes its supplied `callsite` value to an ordinary `SourceLocation` parameter. | The ordinary parameter observes the same runtime value. | Run case. |
| S4 | A call-site-aware function is invoked through a function value from a non-call-site-aware function. | The callee observes the indirect call expression. | Run case. |
| S6 | Source is generated and has an origin mapping. | The exposed location is the mapped user location. | Generated-source fixture. |
| S7 | A package is checked from two different absolute roots. | Exposed package, module, and file values are identical and contain neither root. | Relocation test. |
| S8 | LSP and MCP present the declaration. | Each service identifies the modifier and built-in local variable. | LSP and MCP cases. |
| S9 | A trace retains a `callsite` value after its originating function returns. | Later observation reports the captured location without walking the current stack. | Deferred-observation run case. |
| S10 | A call site contains non-ASCII text before and within its source span. | Lines and columns are one-based, columns count Unicode scalar values, and offsets count UTF-8 bytes. | Run case with exact start and end coordinates from a checked source fixture. |

## Verification and Promotion

Remaining implementation must extend callable lowering, backend metadata, LSP,
and MCP. The run harness must compare locations and their coordinate units
against a checked source fixture rather than machine paths.

After runtime implementation, the source and execution specifications must
explain call-site propagation and generated-source mapping.

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
