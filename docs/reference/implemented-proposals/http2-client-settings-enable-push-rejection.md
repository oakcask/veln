# HTTP/2 Client SETTINGS_ENABLE_PUSH Rejection

Status: implemented

This record preserves the completed endpoint-role SETTINGS receive slice from
the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/run-json.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

The SETTINGS receive transition distinguishes client and server endpoints. A
client rejects peer-sent identifier `2`, `SETTINGS_ENABLE_PUSH`, at the
offending six-byte item offset before applying peer-advertised settings or
other state derived from that frame. The typed
`http2.protocol.settings_not_allowed_for_endpoint` failure carries the setting
identity, endpoint role, SETTINGS frame kind, active state, rule provenance,
and a bounded preview of the inspected item.

The server path continues to accept `SETTINGS_ENABLE_PUSH` values `0` and `1`.
Unknown identifiers, SETTINGS ACK behavior, supported setting range checks,
and local disable-push state are unchanged.

## Evidence

- Historical aggregate evidence checks rejection
  after an earlier valid item in the same frame without applying either item,
  and checks the same multi-item frame remains accepted by a server endpoint.
- `../../../examples/specification/run/http2-protocol-core-settings-enable-push-role-json/`
  checks the structured diagnostic fields and source-visible runtime value.
- `../../../examples/specification/run/http2-protocol-core-settings-enable-push-role-human/`
  checks the focused primary message and related context.
