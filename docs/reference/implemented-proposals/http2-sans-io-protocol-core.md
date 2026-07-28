# HTTP/2 Standard Library Completion and Fixture Retirement

Status: implemented

Current behavior is specified by
[`http2.md`](../../specification/http2.md) and its focused executable routes.
This record preserves the completion boundary for the retired broad fixture.

Reusable connection, stream, HPACK, receive, send, flow-control,
content-length, shutdown, and output-buffer behavior is owned by
`std::http2::core` and `std::http2::hpack`. Transitions are immutable, failure
decisions preserve caller-owned state and output, and production receive and
send paths use the public HPACK codec.

The broad `http2-protocol-core` implementation and case were removed after
their reusable responsibilities moved to standard-owned modules and focused
cases. Its retained route contains no reusable implementation, manifest, or
migration-only executable evidence. Current HTTP/2 verification no longer
depends on the historical fixture revision, retirement inventory, retirement
checker, generated retirement tests, or output-evidence harness.

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
  broad fixture or migration inventory.

The completed cleanup removed the migration-only checker, historical
retirement manifest, and retirement-output standard-package test. Future
HTTP/2 changes should update `http2.md`, the focused specification cases, and
the public standard-package tests directly.
