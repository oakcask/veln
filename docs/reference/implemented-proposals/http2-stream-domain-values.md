# HTTP/2 Stream Domain Values

Status: implemented

This record preserves the completed stream-identifier domain-value slice from
the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/http2.md`, the adjacent standard-library tests, and the
checked executable case under
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The public `std::http2::core` facade keeps the wire schema boundary unchanged:
`UInt31be` stream fields decode to ordinary `Int` values. Before a received
frame reaches stream-state admission, ordinary Veln constructors validate and
wrap real stream identifiers as nonzero `StreamId` values. Client-initiated
and server-initiated constructors enforce the HTTP/2 31-bit maximum and the
endpoint-specific odd or even parity.

`StreamRef` distinguishes the connection stream from `RealStream(StreamId)`.
Connection-only frame validation constructs the connection reference, while
stream-only and promised-stream validation construct real references. Invalid
zero, out-of-range, and wrong-parity inputs retain the existing
`http2.protocol.invalid_stream_id` projection and its required domain,
endpoint role, active state, and rule provenance.

## Evidence

- `../../../crates/veln-stdlib/veln/http2/core.veln` defines the public domain
  values, constructors, and reference classification boundary.
- `../../../crates/veln-stdlib/veln/http2/core_test.veln` checks accepted
  client and server stream ids, retained values, connection and real-stream
  reference projections, and zero, out-of-range, client-parity, and
  server-parity failures.
- `../../../examples/specification/run/http2-protocol-core/main.veln` routes
  connection, client-initiated, and server-initiated validation through the
  public facade before state admission.
- `../../../examples/specification/run/http2-protocol-core/case.toml` checks
  the retained observable stream-id diagnostics, complete stdout, and
  transition ordering.

## Remaining Scope

Flow-control counter domain values, broader lifecycle work, full HPACK,
transport integration, and unrelated raw diagnostic payload fields remain
outside this completed slice.
