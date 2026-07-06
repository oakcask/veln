# HTTP/2 HPACK Huffman Decode Boundary

Status: implemented

This record preserves the completed source-visible HPACK Huffman receive
boundary slice from the HTTP/2 sans-I/O protocol-core proposal. Current
behavior is specified by `../../specification/execution.md`,
`../../specification/commands.md`, and `../../specification/run-json.md`, and
checked by executable examples under `../../../examples/specification/run/`.

## Completed Behavior

The HTTP/2 receive path now routes a checked Huffman-marked static-name
literal value through the source-visible `hpack_static` boundary before the
HPACK fixture fallback. The promoted accepted path decodes the HPACK Huffman
string literal into the visible ASCII value `test` for `:path` and carries the
decoded header through the ordinary request header-list path.

The accepted slice is checked on both completed HEADERS and final CONTINUATION
header blocks, preserving the same immutable HPACK receive-state behavior and
header-block byte accounting used by the surrounding fixture paths.

Malformed Huffman padding, EOS-as-symbol, and non-visible decoded outputs that
are projected from the static boundary keep the focused HPACK fixture ids and
the existing byte-offset, observed-size, observed-first-byte, expected-fixture,
and bounded-preview fields. Their source-visible static projection uses
`codec_module: "hpack_static"` so command-facing JSON and human diagnostics
identify the boundary that rejected the input.

This slice does not add full HPACK compression, unbounded dynamic-table
behavior, or general Huffman acceptance beyond the checked source-visible
static-name receive path.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks
  Huffman-marked `:path: test` through completed HEADERS and final
  CONTINUATION before fixture fallback, including the completed header-block
  bytes.
- `../../../examples/specification/run/http2-protocol-core-hpack-huffman-padding-json/`
  checks the JSON projection for a source-visible static Huffman padding
  failure.
- `../../../examples/specification/run/http2-protocol-core-hpack-huffman-padding-human/`
  checks the human projection for the same source-visible static Huffman
  padding failure.
- `../../../examples/specification/run/http2-protocol-core-hpack-huffman-eos-json/`
  and `../../../examples/specification/run/http2-protocol-core-hpack-huffman-eos-human/`
  check the JSON and human projections for the source-visible static
  EOS-as-symbol failure.
- `../../../examples/specification/run/http2-protocol-core-hpack-huffman-non-visible-json/`
  and
  `../../../examples/specification/run/http2-protocol-core-hpack-huffman-non-visible-human/`
  check the JSON and human projections for the source-visible static
  non-visible decoded-output failure.
