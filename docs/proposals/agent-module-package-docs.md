# Agent Module, Package, And Documentation Model

Status: proposed

This page collects remaining module, package, and generated documentation work
that is outside current implemented behavior. Proposal text here is not current
language behavior unless `../specification/` also states it.

## Read First

- Current source surface:
  [../specification/source-surface.md](../specification/source-surface.md).
- Current command behavior:
  [../specification/commands.md](../specification/commands.md).
- Current specification boundary:
  [../specification/overview.md](../specification/overview.md).

## Current Boundary

Source `mod` declarations own compiler-visible module identity. `use`
declarations import source modules that are part of the analyzed program. The
implemented manifest subset validates selected `[modules]` entries against
source module declarations, requires source ownership for selected manifest
module names, and does not add unselected manifest paths to command source
selection. It does not define package metadata, full discovery semantics, or
additional manifest fields.

Documentation line comments support executable doctests and ADR-lite metadata.
The current toolchain does not expose a `doc` command or generated
documentation workflow as implemented behavior.

Public API boundaries are `pub fn` declarations. Dedicated export lists are
not implemented.

## Proposed Targets

- Define package and tool metadata that belongs in a manifest rather than in
  source.
- Define any future manifest-backed discovery beyond selected-entry validation
  without letting manifest entries rename source modules.
- Add generated documentation behavior that derives from source comments,
  contracts, examples, doctests, and ADR-lite metadata.
- Decide whether dedicated export lists are needed beyond `pub fn`.
- Keep any newly duplicated source and manifest facts checked for drift.

## Non-Targets

- Do not treat generated documentation as canonical language syntax.
- Do not use proposal text to infer package layout or manifest behavior beyond
  the implemented `[modules]` validation.
- Do not move package metadata into source unless the current specification is
  changed after implementation.
