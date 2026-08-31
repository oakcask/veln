---
role: implementation-record
authority: supporting
update-when: The source-less compiler lookup registry descriptors, source-less registry failure details, publication boundary, focused veln-syntax parser evidence, or focused veln-sema registry tests change.
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
Every source-less descriptor module segment and leaf name must be consumable
by the source parser for the concrete lookup route that the descriptor
publishes: the initial byte must satisfy the declared name class, remaining
bytes must be ASCII letters, ASCII digits, or `_`, and the complete segment
must not be a source keyword. Bare prelude lookup keys must also stay name
paths after parser-level literal interpretation, so compiler-provided
descriptors cannot publish bare `true` or `false` prelude routes. Qualified
routes can still publish those leaf spellings when the parser represents the
complete route as a name path.

Invalid compiler-provided descriptors fail registry construction atomically with
the span-less `toolchain.invalid_symbol_case` internal failure. Details include
`provider`, `name`, `name_class`, and `required_initial`. Invalid descriptors
or duplicate lookup keys do not produce source `name.invalid_case` diagnostics.
Diagnostic-producing command and adapter entry points stop before normal
semantic lookup, while production lookup helpers remain unavailable when the
shared registry fails validation. The diagnostic kind is `toolchain`, so
command and adapter outputs keep the failure separate from source name
diagnostics. Invalid lookup-key failures and duplicate lookup-key failures keep
the same diagnostic id and details. Invalid lookup-key primary messages state
that the source lookup key is invalid. Duplicate lookup-key primary messages
state that the lookup key is duplicated.

Qualified runtime lookup keys are the exact module-name and symbol-name pair.
Runtime descriptor modules are single source lookup segments; a runtime module
string that would publish a three-or-more-segment source lookup key fails
publication as an invalid lookup key. Prelude, compiler-adapter, built-in
type-syntax, built-in ADT type, and built-in ADT constructor leaves containing
`::`, `-`, or any other spelling that source lookup cannot produce as one
identifier segment also fail publication as invalid lookup keys. Bare prelude
or public compiler-adapter leaves that parse as contextual literals fail
publication as invalid lookup keys.
Prelude lookup keys are the exact source prelude name. Compiler-adapter
failures report the `compiler_adapter` provider and use `prelude_builtin::name`
lookup keys. The implicit `prelude` module name reports the `standard_names`
provider, built-in type spellings report the `type_syntax` provider, and
built-in ADT type and constructor descriptors report the `adt` provider. A
failure in any one provider publishes no lookup state from the other
source-less providers. Shared standard-environment
initialization validates the registries before publishing reusable command or
adapter state. `prelude_builtin` lookup consumes validated published registry
state, and the prelude-builtin module key is validated even when there are no
compiler-adapter descriptors to publish. Production built-in ADT lookup seeds
application registries from the published built-in ADT registry in the same
shared source-less publication result that owns standard-symbol lookup state.

Compiler temporaries and bookkeeping-only names that cannot participate in
source lookup stay outside the source lookup validation gate. Embedded Veln
prelude sources remain source-written and continue to use ordinary source
casing diagnostics.

Type annotation parsing and public type-annotation reference helpers consume
the published built-in type-syntax registry when checking built-in constructor
arity. A failure in another source-less provider, such as a built-in ADT
descriptor failure, blocks publication before those helpers can use
type-syntax state. A type-syntax descriptor with invalid casing or a duplicate
lookup key also blocks publication before the parser can use that state.
Qualified `prelude` helper lookup consumes the published standard module key,
and qualified `prelude_builtin` helper lookup consumes the published
prelude-builtin module key. Neither helper selects source-less descriptors
through a registry-external module spelling.

## Evidence

- Current behavior route:
  [../../specification/source-less-lookup.md](../../specification/source-less-lookup.md).
- Focused executable evidence:
  `veln-syntax` parser tests and `veln-sema` `standard_symbols`, `adt`, and
  `source_less_lookup` tests for bare and qualified parser name-path
  interpretation, the generated tables, injected invalid descriptors, invalid
  lookup keys,
  duplicate lookup keys, standard-symbol class and lookup-namespace
  mismatches, invalid standard module keys, invalid prelude-builtin module
  keys, atomic failure,
  cross-provider publication failure, checked lookup, type-syntax publication
  consumption, prelude-builtin module-key publication consumption, production
  provider inventory, and lookup isolation. Injected descriptor tests cover
  qualified separators and other non-identifier characters in runtime, prelude
  or compiler-adapter, built-in type-syntax, built-in ADT type, and built-in
  ADT constructor leaves. The `adt` and
  `source_less_lookup` tests also pin that production built-in ADT lookup
  consumes the published built-in ADT registry from the shared publication
  result before constructing application registry state. The
  `source_less_lookup` tests pin cross-provider failure between built-in ADT
  and public type-annotation reference lookup, and they pin published module
  key consumption for standard `prelude` helper lookup, core prelude helper
  lookup, prelude effect lookup, and `prelude_builtin` qualified helper
  lookup. The ordinary Rust test suite keeps source-less publication
  validation checked.
- Public CLI fixtures are not practical for invalid compiler-provided
  descriptor input because that input is not expressible as Veln source or as
  a public command-line option. Serializer and adapter tests consume the
  shared `toolchain.invalid_symbol_case` diagnostic constructor, while
  `veln-sema` registry tests inject invalid descriptors at the publication
  gate.

This record completes only the source-less lookup descriptor acceptance row of
the identifier-casing proposal. Module identity, qualified-use, recovery
navigation, repair rename, rename conflict prediction, MCP rename mapping, and
remaining language-service consumer rows remain proposal scope.
