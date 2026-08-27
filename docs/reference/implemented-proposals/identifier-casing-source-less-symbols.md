---
role: implementation-record
authority: supporting
update-when: The source-less compiler lookup registry descriptors, source-less registry failure details, publication boundary, or focused veln-sema registry tests change.
---

# Identifier Casing Source-Less Symbols

## Completed Boundary

Compiler-provided symbols that participate in source lookup now carry an
explicit source-less name class. One shared source-less construction result
validates compiler-provided descriptor module segments, symbol spellings, and
lookup keys before any standard-symbol or built-in ADT lookup state is
published. Standard-symbol descriptors publish to function lookup only.
Runtime, prelude, and `prelude_builtin` descriptors whose source-less name
class is not `function` fail validation before lookup state is published.

Invalid compiler-provided descriptors fail registry construction atomically with
the span-less `toolchain.invalid_symbol_case` internal failure. Details include
`provider`, `name`, `name_class`, and `required_initial`. Invalid descriptors
or duplicate lookup keys do not produce source `name.invalid_case` diagnostics.
Diagnostic-producing command and adapter entry points stop before normal
semantic lookup, while production lookup helpers remain unavailable when the
shared registry fails validation. The diagnostic kind is `toolchain`, so
command and adapter outputs keep the failure separate from source name
diagnostics. Duplicate lookup-key failures keep the same diagnostic id and
details, and their primary message states that the lookup key is duplicated.

Qualified lookup keys are the exact module-name and symbol-name pair. Prelude
lookup keys are the exact source prelude name. Qualified `prelude_builtin`
compiler-adapter names and built-in ADT type and constructor descriptors are
included in this gate. A failure in any one provider publishes no lookup state
from the other source-less providers. Shared standard-environment
initialization validates the registries before publishing reusable command or
adapter state.
`prelude_builtin` lookup consumes validated published registry state.
Production built-in ADT lookup seeds application registries from the published
built-in ADT registry in the same shared source-less publication result that
owns standard-symbol lookup state.

Compiler temporaries and bookkeeping-only names that cannot participate in
source lookup stay outside the source lookup validation gate. Embedded Veln
prelude sources remain source-written and continue to use ordinary source
casing diagnostics.

## Evidence

- Current behavior route:
  [../../specification/names-effects.md](../../specification/names-effects.md).
- Focused executable evidence:
  `veln-sema` `standard_symbols`, `adt`, and `source_less_lookup` tests for
  the generated tables, injected invalid descriptors, duplicate lookup keys,
  standard-symbol class and lookup-namespace mismatches, atomic failure,
  cross-provider publication failure, checked lookup, and lookup isolation.
  The `adt` and `source_less_lookup` tests also pin that production built-in
  ADT lookup consumes the published built-in ADT registry from the shared
  publication result before constructing application registry state. The Rust
  CI release registry test keeps source-less publication validation checked in
  release builds.

This record completes only the source-less lookup descriptor acceptance row of
the identifier-casing proposal. Module identity, qualified-use, recovery
navigation, repair rename, rename conflict prediction, MCP rename mapping, and
remaining language-service consumer rows remain proposal scope.
