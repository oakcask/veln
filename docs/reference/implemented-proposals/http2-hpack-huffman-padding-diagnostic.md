# HTTP/2 HPACK Huffman Padding Diagnostic

Status: implemented

This record preserves the completed malformed HPACK Huffman padding diagnostic
slice from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is
specified by `../../specification/execution.md`,
`../../specification/examples.md`, `../../specification/run-json.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

The HPACK fixture boundary recognizes malformed HPACK Huffman padding as a
focused fixture failure instead of folding it into the generic unsupported
header-block path. The stable diagnostic id is
`hpack.fixture.malformed_huffman_padding`.

The diagnostic carries the same HPACK fixture shape as unsupported header
blocks: header-block byte offset, observed first byte, observed header-block
size, expected fixture, codec module, and a bounded byte preview. The HTTP/2
protocol-core fixture reaches this failure from both completed HEADERS and
final CONTINUATION header-block boundaries.

This is a fixture-boundary diagnostic only. It does not add full HPACK Huffman
decoding, dynamic-table behavior, schema syntax, or broader HPACK compression
support.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks malformed
  padding through completed HEADERS and final CONTINUATION paths in the
  aggregate HTTP/2 protocol-core fixture output.
- `../../../examples/specification/run/http2-protocol-core-hpack-huffman-padding-human/`
  checks the command-facing human diagnostic message and related notes.
- `../../../examples/specification/run/http2-protocol-core-hpack-huffman-padding-json/`
  checks the command-facing JSON `protocol_diagnostic` fields and bounded byte
  preview.
- `../../specification/execution.md`, `../../specification/examples.md`, and
  `../../specification/run-json.md` summarize the implemented behavior and
  route readers to the checked examples.
