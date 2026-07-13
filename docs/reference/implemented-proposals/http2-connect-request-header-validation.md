# HTTP/2 CONNECT Request Header Validation

Status: implemented

This record closes the ordinary `CONNECT` request-header validation slice from
`../../proposals/http2-sans-io-protocol-core.md`. Current behavior lives in
`../../specification/execution.md`, `../../specification/commands.md`,
`../../specification/run-json.md`, and the checked examples under
`../../../examples/specification/run/`.

## Implemented Behavior

After decoding a completed HEADERS block or final CONTINUATION block, the
HTTP/2 protocol-core example distinguishes ordinary `CONNECT` from other
requests. An ordinary `CONNECT` request requires `:method: CONNECT` and a
non-empty `:authority`, and it omits `:scheme` and `:path`.

Missing or empty `:authority` and present `:scheme` or `:path` fail through
`http2.protocol.invalid_request_header_list`. Stable failed-header facts name
the specific failed condition, while the header name, decoded header names,
stream context, rule provenance, and header-block preview remain structured.
Rejection preserves HPACK, stream, and output state at the existing
request-header failure boundary.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks accepted
  ordinary `CONNECT` requests after completed HEADERS and final CONTINUATION
  blocks, all four rejected pseudo-header shapes, and rejection-state
  preservation.
- The focused
  `../../../examples/specification/run/http2-protocol-core-request-headers-connect-*/`
  cases check human diagnostics and JSON protocol-diagnostic details for each
  rejected shape.

## Remaining Work

Extended CONNECT, `:protocol`, and SETTINGS negotiation remain outside this
completed slice. Full HPACK compression and unrelated production header-field
validation remain in the active protocol-core proposal.
