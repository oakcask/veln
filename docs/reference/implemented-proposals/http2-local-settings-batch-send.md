# HTTP/2 Local SETTINGS Batch Send

Status: implemented

This record preserves the completed ordered local SETTINGS batch send-intent
slice from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is
specified by `../../specification/http2.md`,
`../../specification/execution.md`, `../../specification/run-json.md`, the
adjacent standard tests, and the checked executable cases named below.

## Completed Behavior

The protocol-core fixture can emit local SETTINGS send-intents containing any
ordered batch of the supported local items:
`SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_ENABLE_PUSH`,
`SETTINGS_INITIAL_WINDOW_SIZE`, `SETTINGS_MAX_CONCURRENT_STREAMS`,
`SETTINGS_MAX_FRAME_SIZE`, `SETTINGS_MAX_HEADER_LIST_SIZE`, and
`SETTINGS_ENABLE_CONNECT_PROTOCOL`.

Accepted batches emit one SETTINGS frame with payload length
`6 * item_count`, kind `4`, flags `0`, stream id `0`, and identifier/value
pairs encoded in caller order. The connection records each accepted batch as
one outstanding local SETTINGS batch. A valid peer SETTINGS ACK clears exactly
the oldest outstanding batch while preserving later outstanding batches.

The constrained local item ranges are:
`SETTINGS_ENABLE_PUSH` accepts `0..1`,
`SETTINGS_ENABLE_CONNECT_PROTOCOL` accepts `0..1`,
`SETTINGS_INITIAL_WINDOW_SIZE` accepts `0..2147483647`, and
`SETTINGS_MAX_FRAME_SIZE` accepts `16384..16777215`.
`SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_MAX_CONCURRENT_STREAMS`, and
`SETTINGS_MAX_HEADER_LIST_SIZE` accept only values representable in the HTTP/2
four-byte unsigned SETTINGS value field. An out-of-range item inside a larger
batch is rejected before output bytes are emitted and before an outstanding
local SETTINGS batch is recorded, and uses
`http2.peer_limit.settings_value_out_of_range` with `local_settings`
provenance.
Client endpoints reject local `SETTINGS_ENABLE_CONNECT_PROTOCOL` with
`http2.protocol.settings_not_allowed_for_endpoint`.

This slice does not add new SETTINGS identifiers beyond the implemented
bounded set, HPACK compression, transport I/O, inbound peer SETTINGS semantics
beyond ACKing the local batch, or public source syntax.

## Evidence

- `../../../crates/veln-stdlib/veln/http2/core_test.veln` checks every
  supported setting kind as one bounded capability, exact accepted bytes,
  ordered multi-item batches, local policy offsets, ACK FIFO integration,
  endpoint role rejection, and immutable failure/output behavior.
- `../../../examples/specification/run/http2-core-local-settings-send/`
  imports `http2::core` from `std` and checks public result and output-chunk
  projections.
- `../../../examples/specification/run/http2-protocol-core/` retains a
  three-item local SETTINGS batch, item-order preservation in the emitted
  bytes, ACK clearing of the multi-item batch while a later batch remains
  outstanding, the no-output range-diagnostic path for an invalid item inside
  a larger batch, and the four-byte unsigned SETTINGS value-field boundary for
  `SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_MAX_CONCURRENT_STREAMS`, and
  `SETTINGS_MAX_HEADER_LIST_SIZE`.
- `../../specification/http2.md`, `../../specification/execution.md`, and
  `../../specification/run-json.md` summarize the current behavior and route
  readers to checked evidence.
