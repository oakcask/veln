# HTTP/2 Outbound Maximum Header List Size

Status: implemented

Current behavior is specified in `../../specification/execution.md` and
checked by the executable cases under
`../../../examples/specification/run/http2-protocol-core/`.

Structured outbound HEADERS and server-side `PUSH_PROMISE` intents apply an
active peer-advertised `SETTINGS_MAX_HEADER_LIST_SIZE` after existing
preflight and HPACK validation. The comparison uses decoded HPACK field-size
accounting instead of encoded block length. A list at the maximum is accepted;
an over-limit list is rejected without committing HPACK state, stream
lifecycle, stream-id high-water state, or output chunks, and the same stream
id can be retried after the limit changes.

The focused human and JSON projections live under
`../../../examples/specification/run/http2-protocol-core-outbound-header-list-human/`
and
`../../../examples/specification/run/http2-protocol-core-outbound-header-list-json/`.
They distinguish the peer-owned outbound maximum from the locally configured
inbound header-list receive policy.
