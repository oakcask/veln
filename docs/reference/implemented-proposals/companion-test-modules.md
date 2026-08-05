---
review-when: The historical scope is superseded, its evidence links become invalid, or current documentation starts relying on this record as authority.
---

# Companion Test Modules

Status: implemented

## Summary

The implemented `.test.veln` companion model gives a test source exact-target
access to private functions, source algebraic data types (ADTs), schemas,
nominal effects, and handlers from its matching production source. The model
keeps the companion and target as separate source and analysis scopes.

This page records the completed proposal. Current behavior is specified by
[Source Surface](../../specification/source-surface.md),
[Commands](../../specification/commands.md),
[Test JSON](../../specification/test-json.md), and
[Editor Support](../../specification/editor-support.md). The executable cases
linked below are the primary evidence.

## Completed Source Model

A source named `X.test.veln` is the test companion for the same-directory
production source `X.veln`. The companion writes a normal `use` declaration
for the target module and uses qualified target paths. The permission changes
visibility lookup only.

The completed private-access surface is bounded to these declaration kinds:

| Declaration kind | Implemented observation | Primary evidence |
| --- | --- | --- |
| Function | The matching companion can call a qualified private function and can observe only its production-established signature and effects | [`companion-private-function-access`](../../../examples/specification/check/companion-private-function-access/), [`companion-private-function-production-inference`](../../../examples/specification/check/companion-private-function-production-inference/), and [`companion-private-function-established-effects`](../../../examples/specification/test/companion-private-function-established-effects/) |
| Source ADT | The matching companion can use a qualified private type and its constructors, including private constructors of a public target ADT | [`companion-private-source-adt-access`](../../../examples/specification/check/companion-private-source-adt-access/) and [`companion-private-source-adt-access`](../../../examples/specification/test/companion-private-source-adt-access/) |
| Schema | The matching companion can use a qualified private schema in supported schema-reference positions | [`companion-private-schema-access`](../../../examples/specification/check/companion-private-schema-access/) and [`companion-private-schema-access`](../../../examples/specification/test/companion-private-schema-access/) |
| Nominal effect | The matching companion can use a qualified private effect in operations, effect lists, function types, and companion-local handlers | [`companion-private-effect-types`](../../../examples/specification/check/companion-private-effect-types/) and [`companion-private-effect-operation`](../../../examples/specification/test/companion-private-effect-operation/) |
| Handler | The matching companion can use a qualified private handler while retaining the target-established handled and retained effects | [`companion-private-handler-access`](../../../examples/specification/check/companion-private-handler-access/) and [`companion-private-handler-established-effects`](../../../examples/specification/test/companion-private-handler-established-effects/) |

## Completed Boundaries

The same checked model establishes these boundaries:

| Boundary | Implemented observation | Primary evidence |
| --- | --- | --- |
| Exact target | A companion cannot use a private declaration from another module | The declaration-specific `wrong-target` human and JSON cases under `../../../examples/specification/check/` |
| Non-transitive access | A companion cannot use private declarations from modules imported by its target | The declaration-specific `non-transitive` cases under `../../../examples/specification/check/` |
| Explicit import and qualification | A missing target `use` or a bare target name does not gain companion lookup | The declaration-specific `missing-import` and `bare-name` cases under `../../../examples/specification/check/` |
| Integration-test separation | `X_test.veln` remains an ordinary module and cannot use companion privileges | The declaration-specific `integration-boundary` cases under `../../../examples/specification/check/` |
| Import isolation | Imports remain scoped to the source that writes them | [`companion-private-function-target-import-isolation`](../../../examples/specification/check/companion-private-function-target-import-isolation/) and [`companion-private-function-companion-import-isolation`](../../../examples/specification/check/companion-private-function-companion-import-isolation/) |
| Production-analysis isolation | Companion uses do not complete target inference or change target effects | [`companion-private-function-production-inference`](../../../examples/specification/check/companion-private-function-production-inference/) and the declaration-specific `established-effects-missing` cases |
| Test-only surface | A companion cannot declare public source surface or expose private target declarations through a public alias | [`companion-public-declaration-json`](../../../examples/specification/check/companion-public-declaration-json/), [`companion-public-declaration-human`](../../../examples/specification/check/companion-public-declaration-human/), and [`companion-private-function-alias-boundary`](../../../examples/specification/check/companion-private-function-alias-boundary/) |
| File and package isolation | Production runs, generated public documentation, package exports, and external dependencies do not treat companions as production modules | [`companion-production-exclusion`](../../../examples/specification/run/companion-production-exclusion/), [`companion-discovery-exclusion`](../../../examples/specification/doc/companion-discovery-exclusion/), and [`manifest-companion-export-json`](../../../examples/specification/check/manifest-companion-export-json/) |

Targetless test discovery and explicit source selection include valid companion
and target peers according to the current command specification. The checked
routes are [`companion-discovery`](../../../examples/specification/test/companion-discovery/),
[`companion-explicit-companion-selection`](../../../examples/specification/test/companion-explicit-companion-selection/),
and [`companion-explicit-target-selection`](../../../examples/specification/test/companion-explicit-target-selection/).

The language server preserves target symbol identity for implemented private
function access. Checked definition and rename evidence lives in
[`companion-private-function-identity`](../../../examples/specification/lsp/companion-private-function-identity/)
and
[`companion-private-function-rename-overlay`](../../../examples/specification/lsp/companion-private-function-rename-overlay/).

## Excluded Declaration Surfaces

The completed proposal does not imply friend access for every syntax item.
Public function, type, and schema aliases are public-only declarations. A test
companion cannot declare them. Top-level `codec` and `pub codec` declarations
are not accepted Veln syntax. These surfaces therefore are not remaining
private declaration kinds and are not part of this completed record.

A future source declaration kind does not automatically receive companion
access. Such work requires its own observable access positions, isolation
boundaries, diagnostics, and executable acceptance evidence.

## Design Rationale

The companion does not receive the target module's identity. Sharing module
identity would merge declaration and import scopes. A test-only import or
call-site constraint could then change target name resolution or inference.
Separate identities preserve the invariant that adding a companion can add
test diagnostics and test cases but cannot change whether the target passes
production analysis.

Friend access is exact and non-transitive for the same reason. It permits
focused tests without enlarging the package's public surface or exposing
private declarations across dependency boundaries.

## Completion Evidence

The repository checks the executable cases through the normal specification
harness. Compiler, semantic-analysis, test-runner, CLI, project, documentation,
and language-server tests supplement those command-visible cases.

Relevant local verification routes are:

```sh
bash scripts/agent-test -p veln-analysis
bash scripts/agent-test -p veln-sema
bash scripts/agent-test -p veln-test
bash scripts/agent-test -p veln-cli
bash scripts/agent-test -p veln-project
bash scripts/agent-test -p veln-lsp
```

The proposal is complete because every declaration kind named in the completed
scope has checked success and boundary evidence, the current specification
routes that evidence, and companion behavior remains isolated from production
analysis and publication.

## Read When

- Reviewing why `.test.veln` has exact-target private access while
  `_test.veln` remains an ordinary integration-test module.
- Auditing why companion and target sources keep separate scopes and analysis
  identities.
- Checking which concrete declaration kinds completed the original proposal.
- Confirming why public aliases and removed codec declarations are not pending
  companion work.
