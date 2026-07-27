# HTTP/2 No-Content Response Lifecycle

Status: implemented

This record closes the inbound client-side final `204` and `304` response
lifecycle slice from `http2-sans-io-protocol-core.md`.
Current behavior lives in `../../specification/execution.md` and the checked
protocol-core examples under `../../../examples/specification/run/`.

## Implemented Behavior

The HTTP/2 receive core retains a validated final response status for `204`
and `304` on direct HEADERS and HEADERS-plus-CONTINUATION paths. HEADERS with
`END_STREAM` close the stream directly. Without `END_STREAM`, the stream
retains a no-content response state.

DATA may terminate that state when it contributes zero application-content
octets. PADDED DATA still debits flow-control credit for its full payload but
counts only its application bytes for the no-content rule. Any nonzero
application content fails through the existing protocol-state diagnostic
boundary with expected length zero, the selected status in active state, and
no-content rule provenance. Informational `1xx` responses remain waiting for
a final response and do not select this state.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks direct
  `END_STREAM` for both statuses, empty and padded zero-content DATA
  termination, final CONTINUATION assembly, nonempty DATA rejection, and
  diagnostic projection.
- `../../../examples/specification/run/http2-protocol-core-no-content-data-human/`
  checks the status-specific primary message and related DATA, stream-state,
  preview, and provenance notes.

## Remaining Work

HEAD request semantics, CONNECT tunnel semantics, broader response-status
rules, trailer policy, outbound response send-intents, and socket integration
remain outside this completed slice.
