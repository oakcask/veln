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
retired route contains no reusable Veln implementation. It retains only the
historical migration manifest, structured scenario projection, dimensioned
coverage report, and checker needed to verify that each retired helper,
stdout, and output-table assertion has replacement evidence. Current HTTP/2
behavior is owned by the public standard modules, focused standard-package
tests, and executable specification cases.

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

The retained retirement gate is checked by
`scripts/check-http2-retirement-evidence`. It inventories the deleted broad
fixture from history, requires item-level evidence for 652 helper invocations,
2,044 exact stdout lines, and 315 output tables, verifies the generated
row-addressable scenario manifest, coverage report, and generated projection
evidence test, and rejects unclassified, stale, or unsupported HPACK fixture
compatibility evidence targets. Historical values remain in the inventory, but
current replacement evidence for unsupported HPACK header blocks must use the
public `std::http2::hpack` failure surface rather than
`hpack.fixture.unsupported_header_block`. Each generated scenario has an
executable projection recipe that binds the historical row to its
focused evidence target, public operation, branch, initial state, concrete
input, expected projection, post-state, output provenance, and diagnostic
precedence rather than relying on an assertion hash alone. Rows that target
`retirement_output_evidence_test.veln` are hashed and checked against the
named test plus the helper bodies reachable from that test, so stdout and
output rows cannot satisfy relevance checks from unrelated tests in the same
file. Historical stdout lines that describe retained output chunks route to
the output-evidence test containing the matching retained output-table
assertion.
`retirement_projection_evidence_test.veln` runs bounded projection chunk
row checks, bounded projection chunk fingerprint checks, bounded projection
chunk digest checks, and the generated aggregate projection manifest digest as
standard package tests. The generated test also
imports the public `http2::core`, `http2::frame`, and `http2::hpack` modules
and executes a small public boundary guard through frame encoding, HPACK
encode/decode, and PING send projection so the evidence cannot compile as a
standalone fixture-only artifact. Each row check carries the operation,
branch, state, setup, input, projection, post-state, output, diagnostic
dimension, and projection hash produced by the structured scenario model. The
standard package test validates each row address, rejects empty dimensions,
requires row projection codes to match their ordinals, and compares each
chunk's generated dimension fingerprint with an expected value, while the
checker rejects stale or substituted row dimensions before regeneration.
Meanwhile,
`retirement_output_evidence_test.veln` validates the historical output bytes
through the production frame and public HPACK codecs without restoring the
broad fixture implementation. Its output evidence remains shared only through
reachable helper code used by the named test.
