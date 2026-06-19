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
static-table names, visible-ASCII Huffman-marked literal header lists for the
same fixture boundary, and the checked request and response pseudo-header
fixture lists used by outbound HEADERS and server-side `PUSH_PROMISE`
send-intents.

Successful fixture encoding produces the opaque header-block `ByteChunk` that
the existing outbound send-intent path already accepts. HEADERS therefore
keeps the same single-frame and CONTINUATION splitting behavior based on the
peer-advertised maximum frame size. Server-side `PUSH_PROMISE` keeps the same
promised-stream payload encoding and CONTINUATION splitting after the fixture
encoder produces its header block.

Unsupported header names, unsupported values, and unsupported value encodings
return typed `HpackFixtureFailure` results from the HPACK fixture boundary.
The checked Huffman-marked non-visible value remains on the raw string
encoding failure path. Those failures are not projected as HTTP/2 protocol
diagnostics by the outbound send-intent helpers.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks
  header-list encoding for static indexed `:method: GET` into outbound
  HEADERS, raw literal `:path: /target` into outbound HEADERS split across
  CONTINUATION frames, Huffman-marked literal `:path: test` into outbound
  HEADERS, Huffman-marked literal `:authority: abc.test` into outbound
  HEADERS, static indexed `:status: 200` and Huffman-marked literal
  `:status: 200` into server-side `PUSH_PROMISE`, one non-visible Huffman
  value encode failure, and one unsupported-header encode failure that remains
  an HPACK fixture result.
- `../../specification/execution.md` and `../../specification/run-json.md`
  summarize the implemented outbound fixture encoder boundary and route
  readers to the checked example.
