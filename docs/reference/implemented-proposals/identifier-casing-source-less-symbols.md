---
role: implementation-record
authority: supporting
update-when: The source-less compiler lookup registries, their failure details, or their focused registry tests change.
---

# Identifier Casing Source-Less Symbols

## Completed Boundary

Compiler-provided symbols that participate in source lookup now carry an
explicit source-less name class. One shared source-less construction result
validates compiler-provided descriptor module segments, symbol spellings, and
lookup keys before any standard-symbol or built-in ADT lookup state is
published.

Invalid compiler-provided descriptors fail registry construction atomically with
the span-less `toolchain.invalid_symbol_case` internal failure. Details include
`provider`, `name`, `name_class`, and `required_initial`. Invalid descriptors
or duplicate lookup keys do not produce source `name.invalid_case` diagnostics
and do not reach lookup. The diagnostic kind is `toolchain`, so command and
adapter outputs keep the failure separate from source name diagnostics.
Duplicate lookup-key failures keep the same diagnostic id and details, and
their primary message states that the lookup key is duplicated.

Qualified lookup keys are the exact module-name and symbol-name pair. Prelude
lookup keys are the exact source prelude name. Qualified `prelude_builtin`
compiler-adapter names and built-in ADT type and constructor descriptors are
included in this gate. A failure in any one provider publishes no lookup state
from the other source-less providers. Shared standard-environment
initialization validates the registries before publishing reusable command or
adapter state.
`prelude_builtin` lookup consumes validated published registry state.
Built-in ADT lookup validates the same descriptor set accepted by source-less
publication before constructing production ADT lookup state, while keeping ADT
registry ownership independent of the combined source-less publisher.

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
  atomic failure, cross-provider publication failure, checked lookup, and
  lookup isolation. The `adt` tests also pin that production built-in ADT
  lookup validates built-in descriptors before constructing registry state.

This record completes only the source-less lookup descriptor acceptance row of
the identifier-casing proposal. Module identity, qualified-use, recovery
navigation, repair rename, rename conflict prediction, MCP rename mapping, and
remaining language-service consumer rows remain proposal scope.
