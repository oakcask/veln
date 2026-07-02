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
before falling back to the HPACK fixture codec, except for literal-with-indexing
forms that must update fixture dynamic-table state. The implemented
source-visible path accepts
single-byte static indexed representations for every HPACK static table entry
from `0x81` `:authority` through `0xbd` `www-authenticate:` through one
source-visible static table lookup. It also accepts literal-without-indexing,
literal-with-indexing, and literal-never-indexed header fields whose name
resolves through the same HPACK static table metadata and whose value is a raw
single-byte-length visible-ASCII string or a bounded Huffman-marked literal
value decoded through the HPACK static Huffman table. The HTTP/2 request and
response paths also accept static-name `content-length` after static request
pseudo-headers or a static response `:status` pseudo-header through
literal-without-indexing, literal-with-indexing, and literal-never-indexed
forms when no later fixture dynamic-table reuse is observed, and pass decoded
values to the existing matching header-list validation and content-length
body-accounting paths. The request path also validates static-name `:scheme`
literal values after a static `:method` and before a static `:path` through
the existing request header-list rule, accepting `http` and `https` and
rejecting other visible ASCII values with
`scheme_value_not_http_or_https` on completed HEADERS and final CONTINUATION
paths.

The slice remains limited to static indexed fields and bounded static-name
literal fields. Unsupported Huffman strings, malformed literal lengths,
dynamic-table indexes, table-size updates, and other fixture-owned bytes fall
back to the HPACK fixture boundary. Stateful HTTP/2 header-block decoding still
routes literal-with-indexing blocks through the fixture decoder
when fixture dynamic-table state must be updated.
Static-table boundary failures remain focused: static-only header blocks whose
bytes name no static-table entry fail with `hpack.static.unsupported_index`,
including the checked standalone source-visible boundary for static table
index `62`.

The later static-name Huffman literal promotion is recorded separately in
[http2-hpack-static-name-huffman-literals.md](http2-hpack-static-name-huffman-literals.md).

The diagnostic uses the existing `RuntimeHpackFixtureDiagnostic(...)` detail
shape so human and `run --json` output carry the byte offset, observed header
block size, first byte, expected static header description, decoder module
`hpack_static`, and bounded byte preview.

## Evidence

- `../../../examples/specification/run/hpack-static-codec-boundary/` checks
  representative standalone static indexed entries across the static table,
  supported request blocks, the focused unsupported-index diagnostic above the
  static table boundary, supported literal-without-indexing,
  literal-with-indexing, and literal-never-indexed values for static-table
  pseudo-header and ordinary-header names beyond the earlier checked subset,
  accepted Huffman-marked `:path: test`, line feed, `hpack-byte-ff`,
  `hpack-bytes-00-ff`, `:method: PUT`, and `:status: 200` values across
  the static-name literal forms,
  unsupported static index classification, malformed literal length fallback,
  and saturated length prefix fallback.
- `../../../examples/specification/run/http2-protocol-core/` checks accepted
  request HEADERS, accepted final CONTINUATION completion, accepted response
  HEADERS, accepted source-visible literal-without-indexing response HEADERS,
  accepted request and response `content-length` literal-without-indexing,
  literal-with-indexing, and literal-never-indexed forms that do not observe
  later dynamic-table reuse, non-decimal `content-length` request and response
  validation, accepted request `:scheme` static-name literal values, rejected
  request `:scheme` static-name literal values across the three static-name
  literal forms and final CONTINUATION completion, and the focused unsupported
  static-index failure through the protocol core.
- `../../../examples/specification/run/hpack-static-core-index-unsupported-human/`
  checks the human diagnostic projection for
  `hpack.static.unsupported_index`.
- `../../../examples/specification/run/hpack-static-core-index-unsupported-json/`
  checks the structured `run --json` protocol diagnostic fields.
