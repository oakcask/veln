# HTTP/2 Extended CONNECT Negotiation

Status: implemented

This record closes the extended CONNECT negotiation slice from
`../../proposals/http2-sans-io-protocol-core.md`. Current behavior lives in
`../../specification/execution.md`, `../../specification/commands.md`,
`../../specification/run-json.md`, and the checked HTTP/2 run examples.

## Implemented Behavior

The sans-I/O core recognizes `SETTINGS_ENABLE_CONNECT_PROTOCOL` (`0x8`),
accepts values `0` and `1`, preserves peer and local setting provenance, and
rejects other values through the existing SETTINGS range boundary. A server
can advertise the setting through the ordered local SETTINGS path. A client
cannot advertise it, and a server cannot receive it from a client; those
rejections preserve state and emit no bytes.

Request validation uses the locally advertised server capability. Extended
CONNECT requires `:method: CONNECT`, exactly one non-empty `:protocol`, and
the required `:scheme`, `:path`, and `:authority`. It is rejected before local
enablement, and `:protocol` is rejected on non-CONNECT requests. Ordinary
CONNECT validation is unchanged. Completed HEADERS and final CONTINUATION
paths share the same validation boundary before HPACK or stream state is
committed.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks setting
  values and roles, ordered local send state, accepted and rejected extended
  CONNECT shapes, SETTINGS ACK capability retention, HEADERS and CONTINUATION
  parity, atomic rejection, and the ordinary CONNECT regression path.
- Focused human and JSON cases check diagnostics for
  unnegotiated extended CONNECT under
  `../../../examples/specification/run/http2-protocol-core-request-headers-extended-not-negotiated-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-request-headers-extended-not-negotiated-json/`;
  non-CONNECT `:protocol` under
  `../../../examples/specification/run/http2-protocol-core-request-headers-protocol-non-connect-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-request-headers-protocol-non-connect-json/`;
  duplicate `:protocol` under
  `../../../examples/specification/run/http2-protocol-core-request-headers-protocol-duplicate-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-request-headers-protocol-duplicate-json/`;
  empty `:protocol` under
  `../../../examples/specification/run/http2-protocol-core-request-headers-protocol-empty-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-request-headers-protocol-empty-json/`;
  missing `:scheme` under
  `../../../examples/specification/run/http2-protocol-core-request-headers-extended-scheme-missing-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-request-headers-extended-scheme-missing-json/`;
  missing `:path` under
  `../../../examples/specification/run/http2-protocol-core-request-headers-extended-path-missing-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-request-headers-extended-path-missing-json/`;
  and missing `:authority` under
  `../../../examples/specification/run/http2-protocol-core-request-headers-extended-authority-missing-human/`
  and
  `../../../examples/specification/run/http2-protocol-core-request-headers-extended-authority-missing-json/`.

## Remaining Work

WebSocket framing and semantics, production transport negotiation, and full
HPACK compression remain outside this completed slice.
