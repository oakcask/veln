# Codec Imported Hand-Written Boundary

Status: implemented

This record preserves the completed imported hand-written codec execution
slice from the superseded codec execution design. Current behavior is
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
`Partial`, and `Invalid` values are returned unchanged. When a `Partial`
result carries caller-owned state, a later qualified call to the same imported
codec item can pass that returned state back to the declaring-module encoder
and observe the later `Encoded` completion.

Importing the codec item does not expose the private implementation function,
does not re-export the schema, does not make a private codec callable, and
does not make bare imported codec names ordinary call targets.

## Evidence

- Imported hand-written codec tests check public `decode with` calls through a
  qualified module path, including `Decoded`, `NeedMore(NeedBytes(...))`,
  `NeedMore(NeedEnd)`, and `Invalid` outcomes.
- `../../../examples/specification/run/codec-imported-decode-need-end-boundary-human/`
  and
  `../../../examples/specification/run/codec-imported-decode-need-end-boundary-json/`
  check imported `NeedMore(NeedEnd)` projection at the closed-input reporting
  boundary.
- Imported hand-written codec tests check public `encode with` calls through a
  qualified module path, including `Encoded`, `Partial`, and `Invalid`
  outcomes. They also check the resume path where a later qualified call to
  the same imported codec returns `Encoded(...)`.
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

The source-level codec route is closed by
[Schema Binary Pattern Boundary](schema-binary-pattern-boundary.md). Current
schema work should use explicit schema operations and ordinary functions.
