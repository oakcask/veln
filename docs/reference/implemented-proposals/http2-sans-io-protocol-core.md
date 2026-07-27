# HTTP/2 Standard Library Completion and Fixture Retirement

Status: implemented

This record closes the finite proposal that retired the broad
`examples/specification/run/http2-protocol-core/` executable fixture.

## Implemented Boundary

HTTP/2 remains opt-in through explicit imports from the toolchain-owned `std`
package. Current behavior is specified by `docs/specification/http2.md` and by
the adjacent standard-library tests under `crates/veln-stdlib/veln/http2/`.

The completed ownership boundary is:

- `http2::frame` owns frame schema, decoding, and validated frame-header
  encoding.
- `http2::diagnostic` owns protocol and peer-limit diagnostic constructors.
- `http2::hpack` owns the public HPACK codec, immutable dynamic table, header
  field model, and ordered header-block encoding and decoding.
- `http2::hpack::diagnostic` owns HPACK diagnostic constructors.
- `http2::core` owns pure sans-I/O connection and stream transitions,
  including preface handling, initial peer SETTINGS, frame validation,
  continuation sequencing, role-aware stream domains and lifecycle,
  SETTINGS application and acknowledgement, receive and send flow control,
  header and content-length validation, inbound frame dispatch, outbound frame
  transitions, graceful shutdown, and output-buffer ordering.

All migrated transitions consume explicit immutable input state and return a
typed action, next state, or failure. Failures expose no partial output or
next state and preserve the caller-owned connection, stream, HPACK,
continuation, flow-control, input, and output state.

## Evidence

Pure reusable behavior is covered in `core_test.veln`, `frame_test.veln`,
`hpack_test.veln`, and `diagnostic_test.veln`. Observable behavior is covered
by focused executable cases under `examples/specification/`.

The broad protocol-core fixture no longer owns reusable implementation or
aggregate assertions. Its previous evidence was split across focused cases for
connection state, stream collection, frame admission, receive dispatch,
receive and send flow control, header-list validation, shutdown, outbound
transitions, output buffering, HPACK encoding and decoding, and human and JSON
diagnostic projections.

The retained `examples/specification/run/http2-protocol-core/README.md` is a
route for old links only. It is not an executable case and contains no fixture
implementation.

## Completion

The proposal is complete because the deletion gate is satisfied:

- no executable broad `http2-protocol-core` case remains;
- no current standard test or focused specification case imports the retired
  directory;
- fixture-owned HPACK compatibility modules and the aggregate HTTP/2 state
  machine were removed;
- production HPACK decode and encode are used through the public
  `http2::hpack` boundary;
- current behavior is routed through `docs/specification/http2.md`; and
- focused executable cases carry observable CLI, human, JSON, result-value, and
  emitted-byte evidence.
