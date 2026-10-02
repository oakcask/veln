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

The declaration, static-checking, and direct-call runtime behavior are
specified in
[Call-site Declarations](../specification/call-site-declarations.md). This
proposal tracks only the remaining indirect-call, runtime-contract,
source-identity, lifetime, and presentation work below.

The remaining source-identity work must make `package` and `module`
disambiguate equal relative paths from different dependencies and must verify
that relocation preserves those identities.

## Remaining propagation

Indirect calls must match the direct-call propagation defined by the current
specification. Devirtualization and inlining must not change the observed
location. The implicit context must not become an explicit override at a call
expression.

A runtime-required predicate that directly calls a call-site-aware function
must propagate the same location that the enclosing function body would
supply.

Completion and signature help must present the `callsite` modifier. Completion
inside the function body must include the built-in local variable.

Function-value and indirect-call ABI metadata must carry the hidden source
location without changing the ordinary callable arity. A function without the
modifier must not expose or use that hidden value.

## Generated and Virtual Sources

When the compiler has an origin mapping, `SourceLocation` identifies the
mapped user source. Otherwise it identifies the generated or virtual source.
The file field uses the same canonical virtual-source naming contract as
diagnostics. Moving a package to another machine must not change the exposed
file value.

## Acceptance Model

| Case | Source form | Required observation | Planned evidence |
| --- | --- | --- | --- |
| S4 | A call-site-aware function is invoked through a function value from a non-call-site-aware function. | The callee observes the indirect call expression. | Run case. |
| S6 | Source is generated and has an origin mapping. | The exposed location is the mapped user location. | Generated-source fixture. |
| S7 | Equivalent packages under two absolute roots contain dependencies with the same package-relative source path. | Exposed `file` values are package-relative or canonical virtual paths, all fields contain neither root and remain identical after relocation, and `package` plus `module` disambiguate the dependency sources. | Relocation and dependency-collision test. |
| S8 | LSP and MCP present the declaration. | Each service identifies the modifier and built-in local variable, and signature help presents the modifier. | LSP and MCP cases. |
| S9 | A trace retains a `callsite` value after its originating function returns. | Later observation reports the captured location without walking the current stack. | Deferred-observation run case. |
| S12 | A runtime-required predicate directly calls a call-site-aware function. | The predicate callee observes the location supplied by the enclosing function's direct caller. | Run contract case. |

## Verification and Promotion

Remaining implementation must extend function-value and runtime-contract call
lowering, ABI metadata, generated-source mapping, lifetime coverage, LSP, and
MCP.

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
