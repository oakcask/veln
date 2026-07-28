# HTTP/2 Standard Library Completion and Fixture Retirement

Status: implemented

Current behavior is specified by
[`http2.md`](../../specification/http2.md) and its focused executable routes.
This record preserves the standard-library migration boundary for the retired
broad fixture and the completed semantic evidence cleanup.

Reusable connection, stream, HPACK, receive, send, flow-control,
content-length, shutdown, and output-buffer behavior is owned by
`std::http2::core` and `std::http2::hpack`. Transitions are immutable, failure
decisions preserve caller-owned state and output, and production receive and
send paths use the public HPACK codec.

The broad `http2-protocol-core` implementation and case were removed after
their reusable responsibilities moved to standard-owned modules and focused
cases. The retained historical inventory, structured scenario manifest,
dimensioned coverage report, checker, retained route, and output-evidence
harness are migration-only artifacts; current HTTP/2 behavior is still owned
by the public standard modules, focused standard-package tests, and executable
specification cases.

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

The migration checker reports checked item-level evidence for 652 helper
invocations, 2,044 stdout lines, and 315 output tables, with zero unclassified
items. It also checks `retirement-scenarios.jsonl` and
`retirement-coverage.tsv` against the regenerated model. The manifest records
one scenario row for each retired assertion source and declares the owning
public operation, branch, initial-state model, ordered setup, concrete input
facts, required post-state, executable projection, endpoint role, result
projection, output provenance, failure atomicity, and diagnostic-precedence
dimensions. The checker rejects duplicate result projections and stale
historical setup, helper-line hash inputs, hash-only stdout inputs, and
duplicate lifecycle or stream-state axes, keeping every row addressable even
when several rows share one executable evidence test. The
retirement-output standard-package test keeps historical output tables tied to
public `http2::frame`, `http2::hpack`, and `http2::core` transitions.
