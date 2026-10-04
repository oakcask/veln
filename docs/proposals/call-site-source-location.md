---
role: proposal
update-when: Canonical virtual-source naming, deferred-observation lifetime coverage, or call-site lowering changes.
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
proposal tracks only the remaining S7 canonical virtual-source naming and S9
lifetime work below. Generated locations need canonical virtual names that do
not expose or depend on a machine-specific source root.

## Acceptance Model

| Case | Source form | Required observation | Planned evidence |
| --- | --- | --- | --- |
| S7 | Generated sources have the same logical virtual-source identity. | Exposed `file` values use the same canonical virtual path and contain no machine-specific source root. | Virtual-source naming tests. |
| S9 | A trace retains a `callsite` value after its originating function returns. | Later observation reports the captured location without walking the current stack. | Deferred-observation run case. |

## Verification and Promotion

Remaining implementation must establish canonical virtual-source naming and
deferred-observation lifetime coverage.

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
