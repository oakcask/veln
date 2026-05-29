# Path Runtime Representation

Status: proposed

This page records the path-representation follow-up left outside the completed
source-visible `Path` and `String` assignment boundary. It is proposal work,
not current language behavior.

## Read First

- Current type boundary:
  [../specification/names-effects.md](../specification/names-effects.md) and
  [file system calls](../specification/names-effects-full.md#file-system-calls)
  plus [process calls](../specification/names-effects-full.md#process-calls).
- Current value and runtime semantics:
  [../specification/names-effects-full.md#value-semantics](../specification/names-effects-full.md#value-semantics).
- Current proposal-level target status:
  [target-selection.md](target-selection.md).
- Source-visible boundary completion notes are summarized below.

## Current Boundary

`Path` is source-visible at implemented `fs` and `process` boundaries, and it
is not assignment-compatible with `String`. The runtime still stores path
values with host strings. That representation detail does not authorize source
assignment compatibility or a public path layout guarantee.

The completed source-visible boundary keeps `Path` on implemented `fs`
parameters and the `process::cwd` result, removes the special `Path` and
`String` assignment bridge, and documents that the types are distinct at
assignment boundaries even while runtime storage uses host strings. Semantic
coverage rejects `String` at `fs::read_to_string` and rejects `process::cwd()`
where `Result(String, ProcessError)` is expected.

## Target

Define a richer runtime representation for `Path` only when an implemented
feature needs path-specific behavior that host-string storage cannot express.

The first target should name one observable requirement, such as preserving an
owned path value across standard-library calls, validating host path
conversion, or carrying platform-specific path metadata through a narrow
runtime boundary.

## Non-Goals

- Do not weaken the current `Path` and `String` assignment boundary.
- Do not expose a public path layout, encoding, or normalization guarantee
  before the specification states it.
- Do not change `fs` or `process` signatures as part of a representation-only
  migration.
- Do not make path behavior depend on repository-local absolute paths.

## Acceptance Checks

- The proposal names one observable path behavior that requires the richer
  representation.
- Semantic tests continue to reject `String` where `Path` is required and
  reject `Path` where `String` is required by assignment compatibility.
- Runtime tests cover the new representation through public standard-library
  behavior, not by inspecting internal storage.
- Specification updates describe only implemented source-visible behavior and
  keep backend or host representation details out of the language contract.

## Update When

- Move implemented behavior into `../specification/` only after tests cover the
  observable path behavior.
- Keep representation experiments out of current behavior until a selected
  target is implemented.
