# Codec Imported Hand-Written Boundary

Status: implemented

This record preserves the completed imported hand-written codec execution
slice from `../../proposals/codec-execution-boundary.md`. Current behavior is
specified by `../../specification/source-surface.md`,
`../../specification/execution.md`, `../../specification/examples.md`, and the
checked executable examples under `../../../examples/specification/`.

## Outcome

A `pub codec` declared in another module can be called through a written
qualified import path when its implemented direction is hand-written with
`decode with` or `encode with`.

For `decode with`, the qualified codec call resolves through the imported
codec item name, accepts the same `ByteView` and `ByteOffset` arguments as the
declaring-module decoder, and forwards the call to the private helper named by
the codec declaration. Valid `Decoded`, `NeedMore`, and `Invalid` values are
returned through the same hand-written decode boundary as same-module codec
calls. `NeedMore(NeedEnd)` remains source-visible through ordinary caller
observation, and the command-facing closed-input boundary projects it as
`codec.incomplete_input`.

For `encode with`, the qualified codec call resolves through the imported
codec item name, accepts the referenced encoder function's parameters, and
forwards them to the private helper named by the codec declaration. `Encoded`,
`Partial`, and `Invalid` values are returned unchanged.

Importing the codec item does not expose the private implementation function,
does not re-export the schema, does not make a private codec callable, and
does not make bare imported codec names ordinary call targets.

## Evidence

- `../../../examples/specification/run/codec-imported-decode-boundary/` checks
  a public imported hand-written `decode with` codec call through a qualified
  module path, including `Decoded`, `NeedMore(NeedBytes(...))`,
  `NeedMore(NeedEnd)`, and `Invalid` outcomes.
- `../../../examples/specification/run/codec-imported-decode-need-end-boundary-human/`
  and
  `../../../examples/specification/run/codec-imported-decode-need-end-boundary-json/`
  check imported `NeedMore(NeedEnd)` projection at the closed-input reporting
  boundary.
- `../../../examples/specification/run/codec-imported-encode-boundary/` checks
  a public imported hand-written `encode with` codec call through a qualified
  module path, including `Encoded`, `Partial`, and `Invalid` outcomes.
- `../../../examples/specification/check/codec-imported-private-boundary/`
  checks that a private imported hand-written codec remains unavailable
  through the qualified module path.
- `../../../examples/specification/check/codec-imported-private-implementation-boundary/`
  checks that the private schema and private helper behind an imported public
  hand-written codec remain unavailable as callable items.
- The semantic lowering tests in `veln-sema` check that imported public
  hand-written codec decode and encode calls resolve to the declaring-module
  helper functions, and that a bare imported codec call remains unresolved.

## Remaining Work

The broader codec execution boundary proposal remains open for schema-driven
codec execution beyond the implemented hand-written and helper-backed slices.
