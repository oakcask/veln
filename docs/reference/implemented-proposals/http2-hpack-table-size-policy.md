# HTTP/2 HPACK Table Size Policy

Status: implemented

This record preserves the completed local receive-limit policy slice for HPACK
dynamic table-size updates in the HTTP/2 sans-I/O protocol-core proposal.
Current behavior is specified by `../../specification/execution.md`,
`../../specification/run-json.md`, `../../specification/names-effects-full.md`,
and the checked executable cases under
`../../../examples/specification/run/`.

## Completed Behavior

The HTTP/2 protocol core keeps HPACK integer decoding and bounded fixture
eviction in the HPACK fixture module, then applies the local receive policy
after a completed HEADERS or final CONTINUATION header block returns its next
immutable HPACK state. A decoded dynamic table-size update whose requested
table size is at or below the active local header-table-size receive limit is
accepted and carried into later header-block decoding. A decoded update above
that limit is rejected before the decoded HPACK state is installed, including
when the requested size repeats the current fixture table size.

The rejection is a protocol peer-limit failure, not an HPACK fixture
unsupported-header-block failure. It uses
`http2.peer_limit.header_table_size_exceeded` with the header-block byte
offset, observed requested table size, allowed table size, frame kind, stream
id, receive-limit provenance, and rule provenance. Peer-advertised
`SETTINGS_HEADER_TABLE_SIZE` remains outbound peer state and is not cited as
the inbound receive-limit provenance.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/case.toml` checks
  accepted table-size updates through completed HEADERS and final CONTINUATION
  paths, including the boundary value `160`, and rejects a larger checked
  update through both paths, including a repeated initial fixture table size.
- `../../../examples/specification/run/http2-protocol-core-header-table-human/case.toml`
  checks the human diagnostic projection for the local header-table receive
  limit.
- `../../../examples/specification/run/http2-protocol-core-header-table-json/case.toml`
  checks the structured `run --json` protocol diagnostic fields.
- `../../../examples/specification/run/hpack-fixture-codec-boundary/case.toml`
  remains the focused HPACK fixture evidence for integer decoding and bounded
  dynamic table eviction independent of the HTTP/2 policy boundary.
