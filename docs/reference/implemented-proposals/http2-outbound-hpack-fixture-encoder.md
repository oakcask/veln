# HTTP/2 Outbound HPACK Fixture Encoder

Status: implemented

This record preserves the completed outbound HPACK fixture encoder slice from
the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/run-json.md`, and the
checked executable case
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The imported HPACK fixture module exposes a source-visible header-list encoder
for the outbound protocol-core fixtures. It accepts the fixture-owned
static-indexed header lists already accepted by the decoder, raw
literal-without-indexing and literal-with-indexing header lists for supported
static-table names, checked Huffman-marked literal header lists for the
same fixture boundary, and the checked request and response pseudo-header
fixture lists used by outbound HEADERS and server-side `PUSH_PROMISE`
send-intents.

The fixture module also exposes a stateful encode transition. Callers create a
separate initial encode state, encode a supported literal-with-indexing header
list, and receive both the encoded header block and a new state whose bounded
dynamic table contains the inserted entry. A later matching header list encoded
from that returned state uses the checked dynamic indexed byte `0xbe` and
returns a state with the encode count advanced again. The stateless
`encode_header_list` wrapper remains as a compatibility path that delegates to
the stateful encoder with a fresh initial encode state.

Successful fixture encoding produces the opaque header-block `ByteChunk` that
the existing outbound send-intent path already accepts. HEADERS therefore
keeps the same single-frame and CONTINUATION splitting behavior based on the
peer-advertised maximum frame size. The checked stateful HEADERS case encodes
the HPACK header block before frame splitting: the first literal-with-indexing
block is split across HEADERS and CONTINUATION frames when the peer frame-size
limit is small, while the later matching header list from the returned encode
state is emitted as a single dynamic indexed HEADERS block. Server-side
`PUSH_PROMISE` keeps the same promised-stream payload encoding and
CONTINUATION splitting after the fixture encoder produces its header block.

The stateful encoder also accepts bounded dynamic table-size update requests
for outbound HEADERS header blocks. It emits canonical checked fixture bytes
for the implemented HPACK integer boundary, including `0x3e` for table size
`30` and `0x3f 0x81 0x01` for table size `160`, and returns a new immutable
fixture state with the updated table capacity. Later outbound HEADERS
encoding from that reduced state observes the new capacity before deciding
whether a supported header list can reuse a dynamic indexed entry. A requested
table-size update greater than the active peer-advertised
`SETTINGS_HEADER_TABLE_SIZE` returns a typed HPACK fixture encode failure
before the send-intent path emits header-block bytes.

Unsupported header names, unsupported values, and unsupported value encodings
return typed `HpackFixtureFailure` results from the HPACK fixture boundary.
The checked Huffman-marked single-NUL `:path` fixture value encodes to
`0x04 0x82 0xff 0xc7`, and the checked full-table single-byte
`hpack-byte-ff` `:path` fixture value encodes to
`0x04 0x84 0xff 0xff 0xfb 0xbf`. Later fixture label support accepts
multi-byte `hpack-bytes-*` Huffman-marked values; the checked
`hpack-bytes-00-ff` `:path` fixture value encodes to
`0x04 0x85 0xff 0xc7 0xff 0xff 0xdd`. Raw non-visible string values still
remain on the raw string encoding failure path. Those failures are not
projected as HTTP/2 protocol diagnostics by the outbound send-intent helpers.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks
  header-list encoding for static indexed `:method: GET`, `:path: /`,
  `:scheme: https`, and `:status: 200` into outbound HEADERS, raw literal
  `:path: /target` into outbound HEADERS split across CONTINUATION frames,
  stateful literal-with-indexing `:path: /target`
  encoding before HEADERS splitting, stateful dynamic indexed reuse as
  `0xbe`, outbound dynamic table-size update bytes `0x3e` and
  `0x3f 0x81 0x01`, a following literal HEADERS block that observes reduced
  dynamic-table capacity, an over-peer-limit table-size update failure,
  Huffman-marked literal `:path: test` into outbound HEADERS,
  Huffman-marked literal `:path: hpack-byte-ff` into outbound HEADERS,
  Huffman-marked literal `:path: hpack-bytes-00-ff` into outbound HEADERS,
  Huffman-marked literal `:authority: abc.test` into outbound HEADERS, static
  indexed `:status: 200` and Huffman-marked literal `:status: 200` into
  server-side `PUSH_PROMISE`, one non-visible Huffman value encode failure,
  and one unsupported-header encode failure that remains an HPACK fixture
  result.
- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  the same stateful encoder transition directly at the HPACK fixture boundary:
  separate initial encode state, literal-with-indexing insertion, dynamic
  indexed reuse, encode-count advancement, stateless wrapper compatibility,
  accepted outbound table-size update bytes, reduced table capacity observed
  by a later encode, and over-peer-limit table-size update failure.
- `../../../examples/specification/run/hpack-fixture-codec-json/` checks the
  direct static-indexed header-list encoder bytes for `:method: GET`,
  `:path: /`, `:scheme: https`, and `:status: 200`, plus unsupported
  header-name and header-value failures with expected fixture
  `fixture header list encoding`.
- `../../specification/execution.md` and `../../specification/run-json.md`
  summarize the implemented outbound fixture encoder boundary and route
  readers to the checked example.
