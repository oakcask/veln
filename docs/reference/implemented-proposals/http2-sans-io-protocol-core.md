# HTTP/2 Standard Library Completion and Fixture Retirement

Status: implemented

This record closes the umbrella HTTP/2 sans-I/O core migration. Current
behavior lives in `../../specification/http2.md` and the focused executable
cases under `../../../examples/specification/`.

The broad `http2-protocol-core` fixture is retired. Its retained directory now
contains only a route for old links, while standard-owned behavior is covered
by responsibility-named `http2::core` and `http2::hpack` modules plus focused
examples.

## Completion Evidence

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

Do not use the retired `http2-protocol-core` route as current behavior. Use
`../../specification/http2.md` first, then the focused executable cases named
from that page.
