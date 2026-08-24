---
role: proposal
update-when: The proposed Veln identifier casing remainder, prerequisites, acceptance evidence, or implementation status changes.
---

# Identifier Casing

## Summary

Separate type-like and value-like source names by their first ASCII letter
across all user-visible naming surfaces.

The implemented source-recovery foundation is specified by
[Names And Effects](../specification/names-effects.md),
[Diagnostics JSON](../specification/diagnostics-json.md), and the checked
identifier-casing examples under `../../examples/specification/`. Its
completion history is preserved in
[Recovery-Aware Source Identifier Casing](../reference/implemented-proposals/identifier-casing-source-recovery.md).

This umbrella proposal now tracks only the remaining identifier-casing work.
The next selectable prerequisite is
[Identifier Casing Selection Boundaries](identifier-casing-selection-boundaries.md).
That proposal must be implemented and promoted before additional rows from
this umbrella move into Ready.

## Remaining Naming Contract

The completed foundation covers source ADT type declarations, source ADT
constructors, function declarations, test declaration names, public function
and type aliases, and value bindings inside the `check` and `run` boundaries
named in the current specification.

The remaining work extends the same first-ASCII-letter rule to surfaces that
are not completed by that foundation:

| Remaining surface | Required result | Planned evidence |
| --- | --- | --- |
| Written module identities, import path segments, and import aliases. | Each source-written module segment must start with an ASCII lowercase letter. Invalid aliases must not enter the normal import namespace. | Module and import human and JSON fixtures with exact segment spans. |
| Source-path-derived module identities. | Each user-controlled origin path segment must start with an ASCII lowercase letter. Invalid derived identities must not be registered as normal or importable modules. | Regular, exported, companion, doctest, generated-source, import, duplicate, documentation, metrics, and LSP source-kind fixtures. |
| Qualified uses and patterns. | Each syntax- or resolution-fixed segment must be validated by its semantic role. Unresolved intermediate segments must not be guessed from capitalization. | Expression, pattern, type, alias-target, definition, reference, and rename decision tables. |
| Public function- and type-alias target leaves. | A public function-alias target leaf must be a function name. A public type-alias target leaf must be a type name. Public schema-alias targets remain casing-neutral. | Ordered function-, type-, and schema-alias target fixtures covering wrong case, target kind, unresolved targets, and recovery quarantine. |
| Rename and prepare-rename. | Rename must reject class-changing requested spellings and must repair a unique quarantined declaration only with a class-correct spelling. | Shared language-service, LSP error-mapping, and planned MCP error-mapping cases. |
| Source-less lookup descriptors. | Source-visible compiler-provided symbols must carry an explicit name class and valid spelling before the registry is published. Invalid descriptors must fail atomically with `toolchain.invalid_symbol_case`. | Generated-table, injected-descriptor, release-mode, atomic-failure, and lookup-isolation tests. |

The rule does not add a complete CamelCase or snake_case convention after the
first character. It does not rename schemas, effects, handlers, operations,
record fields, type parameters, or hole labels unless a remaining row above
explicitly fixes their role through another name surface.

## Remaining Selection Boundaries

The source-recovery foundation preserves the current `check` and selected-entry
`run` boundaries. The remaining command and consumer boundaries are owned by
the selection-boundaries prerequisite:

| Consumer | Remaining boundary to specify | Planned evidence |
| --- | --- | --- |
| `test` | Selected test and doctest suites block on selected casing errors before backend compilation. | Selected and unselected test and doctest fixtures. |
| `doc` | Selected non-companion documentation and doctest gates block on selected casing errors. | Documentation generation and doctest fixtures. |
| Language service | Captured snapshots and open-document overlays retain diagnostics and recovery navigation without blocking unrelated snapshots. | Snapshot, overlay, definition, references, prepare-rename, and rename cases. |
| Dependencies and companions | Recovery symbols do not cross dependency, companion, or implicit-prelude boundaries. | Loaded and unloaded dependency fixtures, companion fixtures, and prelude precedence cases. |

## Recovery Remainder

Current behavior already quarantines invalid source declarations and bindings
for the completed foundation. Remaining recovery work must preserve that
boundary when it adds module identities, qualified segments, alias targets,
language-service navigation, rename, dependencies, companions, and source-less
registries.

A valid candidate must continue to win over a compatible recovery record.
Multiple compatible recovery records with the same spelling must not select an
arbitrary target. Recovery-derived unresolved, callability, type-origin,
constructor-arity, or exhaustiveness diagnostics must remain suppressed only
when one unique compatible recovery record explains the failure.

Independently provable diagnostics must still accumulate. Same-class,
same-scope duplicates with the original spelling must report the existing
duplicate diagnostic in addition to casing diagnostics.

## Acceptance Model

This umbrella proposal is complete when:

| Acceptance area | Required result |
| --- | --- |
| Selection-boundary prerequisite. | The Ready selection-boundaries proposal is implemented, promoted to current specification, and removed from `docs/proposals/`. |
| Remaining source surfaces. | Module identities, import segments, import aliases, source-path-derived module segments, qualified use segments, alias target leaves, rename requests, and source-less descriptors follow the remaining naming contract above. |
| Recovery and quarantine. | Invalid names never enter normal lookup, imports, public aliases, package snapshots, checked core, typed IR, exports, backend input, or source-less registries except as explicitly quarantined recovery data. |
| Evidence. | Each remaining row has focused executable evidence under `../../examples/specification/` or an equally direct crate-level test when command fixtures are not practical. |
| Documentation lifecycle. | Implemented behavior is stated under `../specification/`, completed history is moved under `../reference/implemented-proposals/`, and only unimplemented work remains in `docs/proposals/`. |

## Non-Goals

- Changing which valid constructors, functions, modules, or bindings are
  visible.
- Changing duplicate-name rules inside one name class.
- Changing value-versus-value shadowing or initializer visibility.
- Providing compatibility aliases for invalid old spellings.
- Defining unrelated MCP or LSP schemas, coordinates, project scope, or
  transport errors beyond the rename failure mapping required by this proposal.
