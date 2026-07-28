# HTTP/2 HPACK Multi-Byte Non-Visible Fixture

Status: implemented

This record preserves the completed multi-byte non-visible HPACK fixture value
label slice from the HTTP/2 sans-I/O protocol-core proposal. Current
behavior is specified by `../../specification/execution.md`,
`../../specification/examples.md`, `../../specification/run-json.md`, and the
checked executable cases
`../../../examples/specification/run/hpack-fixture-codec-boundary/` and
historical aggregate evidence.

## Completed Behavior

The HPACK fixture string-literal boundary represents multi-byte non-visible
Huffman decoded strings with deterministic `hpack-bytes-xx-...-xx` fixture
labels. The existing `hpack-bytes-00-ff` spelling remains the label for
decoded bytes `0x00 0xff`, and the checked two-NUL decode now produces
`hpack-bytes-00-00`. Source-visible fixture encode accepts the same
multi-byte label form for Huffman-marked fixture values; `hpack-bytes-00-ff`
still emits the checked HPACK bytes
`0x04 0x85 0xff 0xc7 0xff 0xff 0xdd` for a `:path` literal.

This is fixture label support for decoded byte sequences, not arbitrary raw
binary string support. Non-visible raw string encoder input stays on the
fixture-owned raw string encoding failure path.

The HTTP/2 protocol-core fixture carries the accepted label through both
completed HEADERS and final CONTINUATION paths before header-list validation.
The outbound HEADERS fixture encoder accepts the same source-visible value and
emits the checked header-block bytes before the existing frame-header encode
and splitting boundary.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  direct HPACK fixture decode of `hpack-bytes-00-ff` as
  `literal-path-bytes-huffman` and direct decode of `hpack-bytes-00-00` as
  `literal-path-two-nul-huffman`.
- Historical aggregate evidence checks
  `hpack-literal-path-bytes-huffman` through completed HEADERS,
  `hpack-literal-path-two-nul-huffman` through completed HEADERS,
  `hpack-literal-path-bytes-huffman-continuation` through final CONTINUATION,
  and `outbound-headers-hpack-bytes-huffman` through the outbound fixture
  encoder path. The checked output chunks include `0485ffc7ffffdd` as the
  header block and `0000070104000000010485ffc7ffffdd` as the emitted HEADERS
  frame.
- `../../specification/execution.md`, `../../specification/examples.md`, and
  `../../specification/run-json.md` summarize the implemented boundary,
  the general multi-byte non-visible label form, and the focused malformed
  HPACK diagnostic ids that remain outside this label path.
