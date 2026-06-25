# HTTP/2 HPACK Multi-Byte Non-Visible Fixture

Status: implemented

This record preserves the completed bounded multi-byte non-visible HPACK
fixture value slice from the HTTP/2 sans-I/O protocol-core proposal. Current
behavior is specified by `../../specification/execution.md`,
`../../specification/examples.md`, `../../specification/run-json.md`, and the
checked executable cases
`../../../examples/specification/run/hpack-fixture-codec-boundary/` and
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The HPACK fixture string-literal boundary accepts one bounded multi-byte
non-visible fixture label: `hpack-bytes-00-ff`. On Huffman-marked decode, the
label represents decoded bytes `0x00 0xff`. On source-visible fixture encode,
the same label emits the checked HPACK bytes
`0x04 0x85 0xff 0xc7 0xff 0xff 0xdd` for a `:path` literal.

This is a single explicit fixture label, not arbitrary binary string support.
Other multi-byte decoded non-visible Huffman strings stay outside the supported
fixture boundary and continue to report
`hpack.fixture.huffman_non_visible_value`. Non-visible raw string encoder input
also stays on the fixture-owned raw string encoding failure path.

The HTTP/2 protocol-core fixture carries the accepted label through both
completed HEADERS and final CONTINUATION paths before header-list validation.
The outbound HEADERS fixture encoder accepts the same source-visible value and
emits the checked header-block bytes before the existing frame-header encode
and splitting boundary.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  direct HPACK fixture decode of `hpack-bytes-00-ff` as
  `literal-path-bytes-huffman` and preserves the focused
  `hpack.fixture.huffman_non_visible_value` path for an unsupported two-NUL
  decoded Huffman value.
- `../../../examples/specification/run/http2-protocol-core/` checks
  `hpack-literal-path-bytes-huffman` through completed HEADERS,
  `hpack-literal-path-bytes-huffman-continuation` through final CONTINUATION,
  and `outbound-headers-hpack-bytes-huffman` through the outbound fixture
  encoder path. The checked output chunks include `0485ffc7ffffdd` as the
  header block and `0000070104000000010485ffc7ffffdd` as the emitted HEADERS
  frame.
- `../../specification/execution.md`, `../../specification/examples.md`, and
  `../../specification/run-json.md` summarize the implemented boundary,
  the remaining unsupported multi-byte non-visible cases, and the focused
  diagnostic id.
