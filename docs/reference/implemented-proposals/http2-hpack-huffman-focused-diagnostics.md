# HTTP/2 HPACK Huffman Focused Diagnostics

Status: implemented

This record preserves the completed focused unsupported-Huffman diagnostic
slice from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is
specified by `../../specification/execution.md`,
`../../specification/examples.md`, `../../specification/run-json.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

The HPACK fixture boundary keeps malformed Huffman padding on the established
`hpack.fixture.malformed_huffman_padding` diagnostic and splits two other
unsupported Huffman inputs away from the generic
`hpack.fixture.unsupported_header_block` path when the fixture decoder can
identify the failed fact.

Huffman EOS used as a decoded symbol projects as
`hpack.fixture.huffman_eos_symbol`. A Huffman string whose decoded bytes fall
outside the checked fixture string boundary projects as
`hpack.fixture.huffman_non_visible_value`.

Both diagnostics carry the same HPACK fixture detail shape as unsupported
header blocks and malformed Huffman padding: header-block byte offset,
observed header-block size, observed first byte, expected fixture, codec
module, and a bounded byte preview. The direct HPACK fixture examples and the
HTTP/2 protocol-core examples both exercise the command-facing JSON and human
diagnostic projections.

This is a fixture-boundary diagnostic slice only. It does not add full HPACK
compression, general HPACK Huffman behavior beyond checked fixture string
literal decoding and encoding, unbounded dynamic-table behavior, or production
header validation.

## Evidence

- `../../../examples/specification/run/hpack-fixture-huffman-eos-human/` and
  `../../../examples/specification/run/hpack-fixture-huffman-eos-json/` check
  the direct HPACK fixture human and JSON projections for EOS-as-symbol input.
- `../../../examples/specification/run/hpack-fixture-huffman-non-visible-human/`
  and `../../../examples/specification/run/hpack-fixture-huffman-non-visible-json/`
  check the direct HPACK fixture human and JSON projections for non-visible
  decoded bytes.
- `../../../examples/specification/run/http2-protocol-core/` checks the
  focused diagnostics through completed HEADERS and final CONTINUATION paths
  in the aggregate HTTP/2 protocol-core fixture output.
- `../../../examples/specification/run/http2-protocol-core-hpack-huffman-eos-human/`
  and `../../../examples/specification/run/http2-protocol-core-hpack-huffman-eos-json/`
  check the HTTP/2 command-facing human and JSON projections for EOS-as-symbol
  input.
- `../../../examples/specification/run/http2-protocol-core-hpack-huffman-non-visible-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-hpack-huffman-non-visible-json/`
  check the HTTP/2 command-facing human and JSON projections for non-visible
  decoded bytes.
- `../../specification/execution.md`, `../../specification/examples.md`, and
  `../../specification/run-json.md` summarize the implemented behavior and
  route readers to the checked examples.
