# HTTP/2 Header-Block Continuation State

Status: implemented

This record preserves the completed header-block continuation state slice from
the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/run-json.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

The HTTP/2 receive state carries pending header-block assembly as connection
decode state rather than schema or HPACK state. A HEADERS frame without
`END_HEADERS` records the owning stream id, starting frame kind, starting byte
offset, and accumulated opaque header-block bytes, then emits no completed
header event while the assembly is pending.

Same-stream CONTINUATION frames append their payload bytes. A CONTINUATION with
`END_HEADERS` completes the combined opaque header block and hands it to the
HPACK boundary or fixture codec. A different frame kind, a different stream
id, or input end while the assembly is pending reports a protocol-state
failure with continuation context: pending stream id, starting frame kind,
starting byte offset, accumulated header-block byte count, and rule
provenance.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` covers successful
  HEADERS plus CONTINUATION assembly, multiple CONTINUATION frames before
  completion, wrong frame kind, wrong stream id, and input end while a header
  block remains pending.
- `../../../examples/specification/run/http2-protocol-core-continuation-json/`
  and `../../../examples/specification/run/http2-protocol-core-continuation-human/`
  check continuation-ordering command output.
- `../../../examples/specification/run/http2-protocol-core-continuation-stream-json/`
  checks the wrong-stream structured details.
- `../../../examples/specification/run/http2-protocol-core-continuation-closed-json/`
  and `../../../examples/specification/run/http2-protocol-core-continuation-closed-human/`
  check input-end diagnostics while a header block remains pending.
