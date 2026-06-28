# Codec Imported Derived Boundary

Status: implemented

This record preserves the completed imported derived codec execution slice
from `../../proposals/codec-execution-boundary.md`. Current behavior is
specified by `../../specification/source-surface.md`,
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

Importing the codec item does not expose the private schema or generated
helper, does not make a private codec callable, and does not make bare
imported codec names ordinary call targets.

## Evidence

- `../../../examples/specification/run/derived-codec-imported-public-decode-boundary/case.toml`
  checks a public imported `derive decode` codec call through a qualified
  module path, including `Decoded`, short-input `NeedMore`, and
  helper-projected `Invalid` outcomes while the schema stays private to the
  declaring module.
- `../../../examples/specification/run/derived-codec-imported-public-encode-boundary/case.toml`
  checks a public imported `derive encode` codec call through a qualified
  module path, including projected `Encoded` output and
  helper-projected `Invalid(EncodeError)` while the schema stays private to
  the declaring module.
- `../../../examples/specification/check/codec-imported-private-boundary/case.toml`
  checks the shared visibility rule that a private imported codec remains
  unavailable through the qualified module path.
- The semantic lowering tests in `veln-sema` check that imported public
  derived codec decode and encode calls resolve to the declaring-module
  generated helper boundaries, and that a bare imported codec call remains
  unresolved.

## Remaining Work

The broader codec execution boundary proposal remains open for schema-driven
codec execution beyond the implemented hand-written and helper-backed slices.
