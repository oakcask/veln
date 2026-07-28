# HTTP/2 SETTINGS Item-Length Validation

Status: implemented

This record preserves the completed SETTINGS payload item-length validation
slice from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is
specified by `../../specification/execution.md`,
`../../specification/run-json.md`, `../../specification/test-json.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

A non-ACK peer SETTINGS frame whose payload length is not a multiple of the
six-byte SETTINGS item width fails before peer-advertised settings state is
updated. The failure uses `http2.protocol.invalid_payload_length` with frame
kind `4`, stream `0`, the observed payload length, expected item width `6`,
active state `connection-control`, rule provenance
`rfc9113_settings_payload_item_length`, and a bounded byte preview for the
offending payload.

Valid non-ACK SETTINGS frames with zero or more complete six-byte items keep
the existing peer-settings update and pending SETTINGS ACK send-intent
behavior. SETTINGS ACK payload-length handling is unchanged.

This slice does not add SETTINGS identifiers, SETTINGS value-range rules,
stream behavior, HPACK behavior, DATA behavior, GOAWAY behavior, PING
behavior, or network adapter behavior.

## Evidence

- Historical aggregate evidence checks invalid
  partial SETTINGS items in the integrated protocol-core fixture while
  preserving accepted complete SETTINGS item behavior.
- `../../../examples/specification/run/http2-protocol-core-settings-item-length-json/`
  checks structured JSON diagnostic projection for the item-width rule.
- `../../../examples/specification/run/http2-protocol-core-settings-item-length-human/`
  checks human diagnostic projection for the same facts.
- `../../specification/execution.md`, `../../specification/run-json.md`, and
  `../../specification/test-json.md` summarize the current behavior and route
  readers to executable evidence.
