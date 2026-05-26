# Opaque Path Boundary Review

Status: complete.

This review covers the selected self-hosting standard library target for moving
the source-visible `Path` boundary away from `String` assignment compatibility.
The proposal remains historical context; current behavior is defined by the
language reference.

## Completion Check

- `Path` remains the source-visible type for the implemented `fs` parameters
  and `process::cwd` result.
- Assignment compatibility no longer has a special `Path` and `String` bridge.
- The type reference states that `Path` and `String` are distinct at
  assignment boundaries even while the runtime stores paths with host strings.
- The names and effects reference states that `String` and `Path` cannot cross
  the `fs` and `process` standard library boundary by assignment
  compatibility.
- Semantic tests cover rejecting `String` at `fs::read_to_string` and
  rejecting `process::cwd()` when a `Result(String, ProcessError)` is expected.

## Residual Scope

The runtime still stores path values with host strings. That remains an
implementation representation detail, not source assignment compatibility. A
future richer path representation can still change the runtime boundary without
reopening the current source-visible compatibility rule.

## Verification

- `cargo test -p veln-sema path` passed.
