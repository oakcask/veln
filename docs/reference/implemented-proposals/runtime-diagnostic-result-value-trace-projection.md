# Runtime Diagnostic Result Value Trace Projection

Status: implemented

This record preserves the completed command recorder cleanup slice from the
runtime diagnostic payload proposal. Current observable behavior is specified
by `../../specification/run-json.md`, `../../specification/commands.md`,
`../../specification/execution.md`, and the checked executable cases under
`../../../examples/specification/run/`.

## Completed Behavior

The JVM command recorder derives plain `EncodeError(...)`,
`DecodeError(...)`, `DecodeErrorWithReason(...)`, and
`DecodeStep::NeedMore(...)` command-facing diagnostic details from the result
value being recorded. These value shapes no longer require a prior
message-keyed registration entry only to attach their stable result details to
the command trace.

Compatibility registration remains for helper context that is not present in
those result values, such as byte-helper context merged into a
`DecodeErrorWithReason(...)` reason and generated helper details that still
carry command facts outside the returned value.

## Evidence

- `crates/veln-backend-jvm/src/tests.rs` checks direct
  `recordResultFailure(Result.err(...))` calls for `EncodeError(...)`,
  `DecodeError(...)`, `DecodeErrorWithReason(...)`, and
  `DecodeStep::NeedMore(...)` values without pre-registering those result
  values. The `DecodeStep::NeedMore(...)` coverage includes both `NeedBytes`
  and `NeedEnd` readiness payloads.
- The same JVM runtime test also checks that helper context registered under a
  `DecodeErrorWithReason(...)` reason is still merged while the result
  diagnostic id, byte offset, and field path come from the returned value.
- Existing run specification cases for generated encode diagnostics and
  codec decode `NeedMore`/`NeedEnd` diagnostics continue to check stable human
  and JSON output.
