# HTTP/2 Content-Length Body Accounting

Status: implemented

This record closes the fixture-marked `content-length` body accounting slices
from `http2-sans-io-protocol-core.md`. Current behavior lives
in `../../specification/execution.md`, `../../specification/run-json.md`, and
the checked examples under `../../../examples/specification/run/`.

## Implemented Behavior

The HTTP/2 protocol-core example carries an accepted request or response
`content-length` value from completed inbound HEADERS or final CONTINUATION
validation into the tracked stream body state.

Inbound DATA updates that state with only DATA application bytes. PADDED DATA
still consumes receive-window credit for the full DATA payload, including the
pad-length byte and padding bytes, but padding does not count toward the body
length. DATA whose accumulated application byte total exceeds the accepted
`content-length` is rejected immediately. A peer `END_STREAM` on DATA or on
the completed header block is rejected when the accumulated DATA application
byte count is still shorter than the accepted `content-length`. Exact matches
are accepted and can close the stream by peer.

Body-length failures project as `http2.protocol.content_length_mismatch` with
the expected content length, observed DATA application byte count, frame kind,
stream id, active state, rule provenance, and a bounded DATA byte preview in
structured JSON or human related notes.

Outbound request and response HEADERS send-intents with accepted
fixture-marked `content-length` values also carry the expected body length
into local outbound send-credit state. Later outbound DATA send-intents count
only DATA application bytes against that expectation, including for PADDED
DATA while the full encoded DATA payload still consumes outbound connection
and stream credit. DATA that would exceed the expected application byte count,
or DATA with local `END_STREAM` before that count is reached, is rejected
before output bytes or send-credit changes through the same
`http2.protocol.content_length_mismatch` shape.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks that
  accepted request and response `content-length` values, including
  source-visible response static-name `content-length` values, are carried
  into stream body state and updated by later inbound DATA. The same
  executable example checks outbound request and response `content-length`
  send-credit tracking,
  accepted exact-length DATA, accepted PADDED DATA, over-length rejection, and
  early local `END_STREAM` rejection.
- `../../../examples/specification/run/http2-protocol-core-content-length-body/`
  checks focused outbound exact-length DATA with local `END_STREAM`, local
  closed-stream transition after the exact match, and PADDED DATA splitting
  where padding consumes outbound send credit without counting toward the body
  length.
- `../../../examples/specification/run/http2-protocol-core-content-length-over-json/`
  checks outbound JSON projection for over-length DATA.
- `../../../examples/specification/run/http2-protocol-core-content-length-early-human/`
  checks outbound human projection for an early local `END_STREAM` shortfall.
- `../../specification/execution.md`, `../../specification/run-json.md`, and
  `../../specification/examples.md` summarize the current behavior and route
  to the checked examples.

## Remaining Work

Full HPACK compression, socket integration, trailer field validation beyond
the checked fixture boundary, and broad multi-stream body accounting remain
outside this completed slice.
