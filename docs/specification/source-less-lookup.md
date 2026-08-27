---
role: specification
authority: normative
update-when: The compiler-provided source-less lookup descriptors, source-less registry failure details, publication boundary, or focused veln-sema registry tests change.
---

# Source-Less Lookup

This page specifies compiler-provided symbols that can participate in source lookup but do not originate from source declarations.

## Source-Less Lookup Registry

Every compiler-provided symbol that participates in source lookup has an explicit source-less name class. Runtime, prelude, compiler-adapter, implicit standard-name module, built-in type-syntax, and built-in ADT descriptors validate their module segments, spelling, declared name class, and lookup key before lookup state is published.

Registry construction is atomic in release and test builds. Valid input publishes one complete immutable source-less lookup registry set. If any provider descriptor is invalid, the shared publication result fails with span-less `toolchain.invalid_symbol_case`, and no lookup state from the other source-less providers is published.

The failure details contain `provider`, `name`, `name_class`, and `required_initial`. The diagnostic kind is `toolchain`; source-less descriptor failures do not produce source `name.invalid_case` diagnostics. Invalid source lookup keys, duplicate lookup keys, and descriptor class mismatches use the same id and detail fields.

Source-less providers expose these lookup keys:

| Provider detail | Lookup key |
| --- | --- |
| Runtime descriptors | `module::name` |
| Prelude descriptors | the exact source prelude helper name |
| Compiler-adapter descriptors reporting `compiler_adapter` | `prelude_builtin::name` |
| The implicit standard module name reporting `standard_names` | `prelude` |
| Built-in type-syntax descriptors reporting `type_syntax` | the built-in type constructor spelling |
| Built-in ADT descriptors reporting `adt` | type and constructor lookup keys, such as `Option` and `Option::Some` |

Normal lookup consumers use the published source-less registry state. Qualified
`prelude::name` helper lookup compares the qualifier against the published
standard module key before selecting a prelude descriptor or classifying a
prelude effect helper. Qualified `prelude_builtin::name` helper lookup
compares the qualifier against the published prelude-builtin module key before
selecting a compiler-adapter descriptor. Public type-annotation reference
helpers and internal type annotation parsing check built-in type constructor
arity through the published built-in type-syntax registry. Built-in ADT lookup
seeds application registry state from the published built-in ADT registry.

Focused `veln-sema` `standard_symbols`, `adt`, and `source_less_lookup` tests are the executable evidence for generated-table validation, injected invalid descriptors, release-mode validation, atomic failure, cross-provider publication failure, checked lookup, provider inventory, and lookup isolation. Public source fixtures cannot inject compiler descriptors, so this contract is verified by focused Rust tests rather than examples under `examples/specification/`.
