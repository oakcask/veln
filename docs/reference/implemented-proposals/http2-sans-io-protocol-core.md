# HTTP/2 Standard Library Completion and Fixture Retirement

Status: implemented

Current behavior is specified by
[`http2.md`](../../specification/http2.md) and its focused executable routes.
This record preserves the standard-library migration boundary for the retired
broad fixture.

Reusable connection, stream, HPACK, receive, send, flow-control,
content-length, shutdown, and output-buffer behavior is owned by
`std::http2::core` and `std::http2::hpack`. Transitions are immutable, failure
decisions preserve caller-owned state and output, and production receive and
send paths use the public HPACK codec.

The broad `http2-protocol-core` implementation was removed after its reusable
responsibilities moved to standard-owned modules and focused cases. The
retired route contains no reusable Veln implementation and no migration-only
inventory. Current HTTP/2 behavior is owned by the public standard modules,
focused standard-package tests, and executable specification cases.

## Completion Evidence

The current behavior is covered by public standard-package tests and focused
executable specification cases:

- `core_test.veln` covers immutable aggregate connection state, stream
  collections, flow-control domains, SETTINGS state, receive-frame dispatch,
  chunked receive state, shutdown, PING, outbound DATA, WINDOW_UPDATE,
  HEADERS, PUSH_PROMISE, RST_STREAM, PRIORITY, and output-buffer ordering.
- `hpack_test.veln` covers HPACK integer, Huffman, dynamic-table, indexed
  field, literal field, table-size update, header-block decoding, and
  header-block encoding behavior.
- Focused `http2-core-*` cases under `examples/specification/run/` record
  public state, branch, diagnostic, and byte projections for the standard
  sans-I/O surface.
- Focused `http2-protocol-core-*` cases remain executable only where their
  human and JSON diagnostic projections are current behavior; they are not a
  broad fixture, migration inventory, or reusable implementation.

The migration-only retirement gate was removed after replacement coverage
became independently executable through the current standard-package tests and
focused specification cases. The workspace no longer reconstructs the deleted
broad fixture from history, retains row-addressable retirement manifests, or
runs generated retirement row tests. Unsupported HPACK header blocks are
covered through the public `std::http2::hpack` failure surface rather than
`hpack.fixture.unsupported_header_block`.
