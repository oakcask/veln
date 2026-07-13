# Codec Imported Derived Boundary

Status: implemented

This record preserves the completed imported derived codec execution slice
from the superseded codec execution design. Source-level `codec`
declarations are no longer current source syntax; current schema operation
behavior is specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

A `pub codec` declared in another module can be called through a written
qualified import path when its implemented direction is generated with
`derive decode` or `derive encode`.

For `derive decode`, the qualified codec call resolves through the imported
codec item name, accepts the bounded `ByteView` and explicit base
`ByteOffset`, and invokes the declaring module's generated
`byte_decode_step_<schema>` helper. The call returns generated-helper-backed
`Decoded`, `NeedMore`, and `Invalid` values through the same derived decode
boundary as same-module codec calls.

For `derive encode`, the qualified codec call resolves through the imported
codec item name, accepts the generated helper's value argument, and invokes
the declaring module's generated `byte_encode_<schema>` helper. Helper
success is projected to `Encoded(List<ByteChunk>)`; helper
`Err(EncodeError)` output is projected to `Invalid(EncodeError)`.
When the call also supplies a `ByteCount` budget, the imported codec uses the
same budgeted derived encode boundary as a same-module codec call: complete
output is projected to `Encoded`, oversized output is projected to
`Partial(List<ByteChunk>, ByteCount, state)`, the returned state can resume a
later import-qualified call, and helper `Err(EncodeError)` is projected to
`Invalid(EncodeError)` before any output chunk is exposed.

Importing the codec item does not expose the private schema or generated
helper, does not make a private codec callable, and does not make bare
imported codec names ordinary call targets.

## Evidence

- Imported derived codec tests check public `derive decode` and `derive encode`
  calls through a qualified module path while the schema stays private to the
  declaring module.
- `../../../examples/specification/check/codec-imported-private-boundary/case.toml`
  checks the shared visibility rule that a private imported codec remains
  unavailable through the qualified module path.
- The semantic lowering tests in `veln-sema` check that imported public
  derived codec decode and encode calls resolve to the declaring-module
  generated helper boundaries, and that a bare imported codec call remains
  unresolved.

## Remaining Work

The source-level codec route is closed by
[Schema Binary Pattern Boundary](schema-binary-pattern-boundary.md). Current
schema work should use explicit schema operations and ordinary functions.
