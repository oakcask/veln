---
role: proposal
update-when: Caller parameters, source-location values, generated-source mapping, or call-site lowering is implemented or redesigned.
---

# Call-site Source Location

Libraries that produce diagnostics, logs, events, and traces need the user's
call site rather than the source location inside a wrapper function. Veln must
provide this without making each observability function a compiler-recognized
special case.

## Outcome

Add the standard `SourceLocation` value, the pure `source::here()` intrinsic,
and one optional caller parameter per function.

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

```veln
source::here() -> SourceLocation
```

`source::here()` evaluates to the span of its own expression.

## Caller Parameters

A function can mark its final parameter with `caller`:

```veln
pub fn info(
  message: String,
  attributes: Attributes,
  caller site: SourceLocation,
) -> () effects [Observe]
  observe::emit(LogInfo(message, attributes, site))
end
```

The proposed grammar addition is:

```ebnf
CallerParameter ::= "caller" Name ":" "SourceLocation"
```

A caller parameter must be the final parameter. Its type must be
`SourceLocation`. A function cannot declare more than one caller parameter.

Callers may omit that final argument. The compiler then supplies the call
expression's location. A caller may also pass an explicit `SourceLocation`.
This permits wrappers to forward the original site:

```veln
pub fn warning(message: String, caller site: SourceLocation) -> () effects [Observe]
  info(message, {}, site)
end
```

An omitted caller argument inside a function that itself has a caller
parameter forwards that parameter. A wrapper passes `source::here()` explicitly
when it intentionally wants its internal call expression instead.

The caller parameter is visible in source, documentation, completion, and
signature help. It does not contribute to the ordinary callable arity or
function type. An indirect call supplies the indirect call expression's
location when no explicit value is forwarded.

The call ABI must therefore carry a hidden source location for direct and
indirect calls. A function without a caller parameter does not expose or use
that hidden value. An implementation can specialize direct calls, but the
observable location cannot depend on whether a call was devirtualized or
inlined.

## Generated and Virtual Sources

When the compiler has an origin mapping, `SourceLocation` identifies the
mapped user source. Otherwise it identifies the generated or virtual source.
The file field uses the same canonical virtual-source naming contract as
diagnostics. Moving a package to another machine must not change the exposed
file value.

## Acceptance Model

| Case | Source form | Required observation | Planned evidence |
| --- | --- | --- | --- |
| S1 | Direct `source::here()` call. | The value covers that expression and uses one-based line and column coordinates. | Run case with a checked source fixture. |
| S2 | Direct call that omits a caller parameter. | The callee receives the caller's call-expression location. | Run specification case. |
| S3 | Wrapper forwards its caller parameter explicitly. | The final callee observes the outer user's site. | Nested-wrapper run case. |
| S4 | Wrapper passes `source::here()` explicitly. | The final callee observes the wrapper's internal call site. | Run specification case. |
| S5 | Caller-aware function is invoked through a function value. | The callee observes the indirect call expression and callable type checking remains unchanged. | Type-check and run cases. |
| S6 | Caller parameter is not final, has the wrong type, or is duplicated. | Checking reports the failed declaration rule and a repair. | Check and check-JSON cases. |
| S7 | Source is generated and has an origin mapping. | The exposed location is the mapped user location. | Generated-source fixture. |
| S8 | A package is checked from two different absolute roots. | Exposed package, module, and file values are identical and contain neither root. | Relocation test. |
| S9 | Formatter, docs, LSP, and MCP present the declaration. | Each surface identifies the caller parameter without counting it as a required ordinary argument. | Formatter, documentation, LSP, and MCP cases. |

## Verification and Promotion

Implementation must extend executable grammar, accepted and rejected source
fixtures, AST wire encoding, semantic analysis, callable lowering, backend
metadata, formatting, documentation, LSP, and MCP. The run harness must compare
locations against a checked source fixture rather than machine paths.

After implementation, the current source-surface and name/effect
specifications must explain caller parameters. The source and execution
specifications must explain `SourceLocation` and generated-source mapping.

## Non-goals

- This proposal does not expose a runtime stack trace.
- Source locations are not stable identifiers across source edits.
- The proposal does not add general optional or default parameters.
- The proposal does not give libraries access to machine-specific source
  paths.
