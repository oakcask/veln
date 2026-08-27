---
role: implementation-record
authority: supporting
update-when: The source-less compiler lookup registries, their failure details, or their focused registry tests change.
---

# Identifier Casing Source-Less Symbols

## Completed Boundary

Compiler-provided symbols that participate in source lookup now carry an
explicit source-less name class. The checked source lookup registries validate
compiler-provided descriptor module segments, symbol spellings, and lookup
keys before they publish lookup state.

Invalid compiler-provided descriptors fail registry construction atomically with
the span-less `toolchain.invalid_symbol_case` internal failure. Details include
`provider`, `name`, `name_class`, and `required_initial`. Invalid descriptors
or duplicate lookup keys do not produce source `name.invalid_case` diagnostics
and do not reach lookup. The diagnostic kind is `toolchain`, so command and
adapter outputs keep the failure separate from source name diagnostics.

Qualified lookup keys are the exact module-name and symbol-name pair. Prelude
lookup keys are the exact source prelude name. Qualified `prelude_builtin`
compiler-adapter names and built-in ADT type and constructor descriptors are
included in this gate. Shared standard-environment initialization validates
the registries before publishing reusable command or adapter state.

Compiler temporaries and bookkeeping-only names that cannot participate in
source lookup stay outside the source lookup validation gate. Embedded Veln
prelude sources remain source-written and continue to use ordinary source
casing diagnostics.

## Evidence

- Current behavior route:
  [../../specification/names-effects.md](../../specification/names-effects.md).
- Focused executable evidence:
  `veln-sema` `standard_symbols` and `adt` tests for the generated tables,
  injected invalid descriptors, duplicate lookup keys, atomic failure, checked
  lookup, and lookup isolation.

This record completes only the source-less lookup descriptor acceptance row of
the identifier-casing proposal. Module identity, qualified-use, recovery
navigation, repair rename, rename conflict prediction, MCP rename mapping, and
remaining language-service consumer rows remain proposal scope.
