---
role: proposal
update-when: Generated-source mapping, relocation-safe source identity, deferred-observation lifetime coverage, or call-site lowering changes.
---

# Call-site Source Location

Libraries that produce diagnostics, logs, events, and traces need the user's
call site rather than the source location inside a wrapper function. Veln must
provide this without making each observability function a compiler-recognized
special case.

The location must be captured when the library API is called. A trace can be
finished or exported after the originating call stack no longer exists, so a
later stack walk cannot recover the required logical call site.

The declaration, static-checking, and direct- and indirect-call runtime
behavior are specified in
[Call-site Declarations](../specification/call-site-declarations.md). This
proposal tracks only the remaining S6, S7, and S9 source-identity and lifetime
work below.

The remaining source-identity work must make `package` and `module`
disambiguate equal relative paths from different dependencies and must verify
that relocation preserves those identities.

## Generated and Virtual Sources

When the compiler has an origin mapping, `SourceLocation` identifies the
mapped user source. Otherwise it identifies the generated or virtual source.
The file field uses the same canonical virtual-source naming contract as
diagnostics. Moving a package to another machine must not change the exposed
file value.

## Acceptance Model

| Case | Source form | Required observation | Planned evidence |
| --- | --- | --- | --- |
| S6 | Source is generated and has an origin mapping. | The exposed location is the mapped user location. | Generated-source fixture. |
| S7 | Equivalent packages under two absolute roots contain dependencies with the same package-relative source path. | Exposed `file` values are package-relative or canonical virtual paths, all fields contain neither root and remain identical after relocation, and `package` plus `module` disambiguate the dependency sources. | Relocation and dependency-collision test. |
| S9 | A trace retains a `callsite` value after its originating function returns. | Later observation reports the captured location without walking the current stack. | Deferred-observation run case. |

## Verification and Promotion

Remaining implementation must extend generated-source mapping and lifetime
coverage.

## Non-goals

- This proposal does not expose a runtime stack trace.
- This proposal does not make a runtime-required predicate in an ordinary
  function construct call-site context for a call-site-aware callee. Only an
  enclosing call-site-aware function has hidden context that its predicate can
  forward.
- Source locations are not stable identifiers across source edits.
- The proposal does not add general optional or default parameters.
- The proposal does not add syntax that overrides implicit call-site context at
  an individual call expression.
- The proposal does not expose an arbitrary current expression's location as a
  `SourceLocation` value.
- The proposal does not give libraries access to machine-specific source
  paths.
