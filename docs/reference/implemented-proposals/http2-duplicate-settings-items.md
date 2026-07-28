# HTTP/2 Duplicate SETTINGS Items

Status: implemented

## Result

The sans-I/O HTTP/2 receive transition processes duplicate known items in a
non-ACK SETTINGS frame in wire order and retains the last occurrence as the
active peer-advertised value. Unknown identifiers interleaved with those items
remain ignored.

Repeated `SETTINGS_INITIAL_WINDOW_SIZE` items apply each ordered delta to every
tracked open outbound stream. Connection credit, content-length body
accounting, and closed or reset stream lifecycle are preserved.

The frame update remains atomic because all item values and endpoint-role rules
are validated before peer state or derived outbound credit is committed. A
later invalid duplicate uses the existing item-local diagnostic and byte
preview at that item's offset while leaving the pre-frame state unchanged.

## Current Behavior

- Execution semantics: [../../specification/execution.md](../../specification/execution.md).
- Run JSON projection: [../../specification/run-json.md](../../specification/run-json.md).

## Executable Evidence

- Integrated state, ordered delta, unknown-item, lifecycle, accounting, and
  rollback coverage:
  historical aggregate evidence.
- Focused human range diagnostic:
  `../../../examples/specification/run/http2-protocol-core-settings-value-human/`.
- Focused JSON range diagnostic:
  `../../../examples/specification/run/http2-protocol-core-settings-value-json/`.
