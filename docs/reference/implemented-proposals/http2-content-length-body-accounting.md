# HTTP/2 Content-Length Body Accounting

Status: implemented

This record closes the fixture-marked `content-length` body accounting slice
from `../../proposals/http2-sans-io-protocol-core.md`. Current behavior lives
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

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks that
  accepted request `content-length` values are carried into stream body state
  and updated by later DATA.
- `../../../examples/specification/run/http2-protocol-core-content-length-body/`
  checks exact DATA application byte matches and PADDED DATA where padding
  consumes receive-window credit without counting toward the body length.
- `../../../examples/specification/run/http2-protocol-core-content-length-over-json/`
  checks JSON projection for over-length DATA.
- `../../../examples/specification/run/http2-protocol-core-content-length-early-human/`
  checks human projection for an early peer `END_STREAM` shortfall.
- `../../specification/execution.md`, `../../specification/run-json.md`, and
  `../../specification/examples.md` summarize the current behavior and route
  to the checked examples.

## Remaining Work

Full HPACK compression, socket integration, trailer field validation beyond
the checked fixture boundary, and broad multi-stream body accounting remain
outside this completed slice.
