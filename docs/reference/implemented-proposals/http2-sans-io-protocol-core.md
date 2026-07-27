# HTTP/2 Standard Library Completion and Fixture Retirement

Status: superseded

This premature umbrella completion record is superseded by the active proposal
at `../../proposals/http2-sans-io-protocol-core.md`. Current implemented
behavior lives in `../../specification/http2.md` and the focused executable
cases under `../../../examples/specification/`.

The broad `http2-protocol-core` fixture is retired. Its retained directory now
contains only a route for old links. That deletion is not completion evidence
for the original matrix gate until every removed helper invocation, exact
stdout line, output table, and focused fixture marker is classified.

## Implemented Evidence

- `http2::core` owns immutable aggregate connection state, stream collection
  state, public HPACK table integration, receive-frame dispatch, ordered
  chunked receive, output-buffer ordering, send transitions, shutdown, and
  failure accessors.
- `http2::hpack` is the only production HPACK codec used by standard-owned
  receive and send transitions. The retired aggregate fixture no longer
  carries fixture codec state or compatibility fallback code.
- `../../../examples/specification/run/http2-core-receive-connection-boundary/`
  records preface plus initial SETTINGS composition, ordered same-chunk
  SETTINGS and PING handling, emitted SETTINGS ACK and PING ACK chunks,
  inbound PRIORITY offset application, initial-gate rejection, and later-frame
  rejection without a partial next state.
- Adjacent standard tests in
  `../../../crates/veln-stdlib/veln/http2/core_test.veln` cover the pure and
  immutable standard-library helpers behind the focused cases.

## Remaining Proposal Gate

The checked replacement matrix for the removed aggregate case now lives in the
active cleanup proposal. Use
`../../proposals/http2-sans-io-protocol-core.md` for the remaining stale-route
cleanup and verification work. Do not use the retired `http2-protocol-core`
route as current behavior. Use `../../specification/http2.md` first, then the
focused executable cases named from that page.
