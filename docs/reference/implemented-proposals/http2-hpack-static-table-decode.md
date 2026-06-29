# HTTP/2 HPACK Static Table Decode

Status: implemented

This record preserves the completed source-visible HPACK static-table decode
slice from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is
specified by `../../specification/execution.md`,
`../../specification/run-json.md`, `../../specification/commands.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

The HTTP/2 protocol core routes complete HEADERS blocks and completed final
CONTINUATION header blocks through the source-visible `hpack_static` decoder
before falling back to the HPACK fixture codec. The implemented subset accepts
static indexed representations for `:method: GET`, `:method: POST`,
`:path: /`, `:scheme: http`, `:scheme: https`, `:status: 200`, and
`:status: 404`. It also accepts literal-without-indexing header fields whose
name is one of the supported static-table indexes `:authority`, `:path`,
`:status`, `server`, `content-type`, or `user-agent`, and whose value is a
raw single-byte-length visible-ASCII string.

The slice remains limited to static indexed fields and static-name
literal-without-indexing fields. Unsupported literal names, Huffman strings,
malformed literal lengths, dynamic-table indexes, table-size updates, and
other fixture-owned bytes fall back to the HPACK fixture boundary. Static-only
header blocks whose bytes are indexed representations but whose static-table
indexes are outside the supported subset fail with the focused
`hpack.static.unsupported_index` diagnostic instead of falling through to the
generic unsupported-header-block path.

The diagnostic uses the existing `RuntimeHpackFixtureDiagnostic(...)` detail
shape so human and `run --json` output carry the byte offset, observed header
block size, first byte, expected static header description, decoder module
`hpack_static`, and bounded byte preview.

## Evidence

- `../../../examples/specification/run/hpack-static-codec-boundary/` checks
  each supported standalone static indexed entry, supported request blocks,
  supported literal-without-indexing pseudo-header and ordinary-header values,
  unsupported static index classification, unsupported literal fallback
  classification, malformed literal length fallback, and saturated length
  prefix fallback.
- `../../../examples/specification/run/http2-protocol-core/` checks accepted
  request HEADERS, accepted final CONTINUATION completion, accepted response
  HEADERS, accepted source-visible literal-without-indexing response HEADERS,
  and the focused unsupported static-index failure through the protocol core.
- `../../../examples/specification/run/hpack-static-core-index-unsupported-human/`
  checks the human diagnostic projection for
  `hpack.static.unsupported_index`.
- `../../../examples/specification/run/hpack-static-core-index-unsupported-json/`
  checks the structured `run --json` protocol diagnostic fields.
