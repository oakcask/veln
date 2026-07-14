# General Inbound HPACK Table-Size Update Sequences

Status: implemented

This record preserves the completed general inbound dynamic-table size update
sequence slice from the HTTP/2 sans-I/O protocol-core proposal. Current
behavior is specified by `../../specification/execution.md` and the checked
executable cases under `../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The production inbound HPACK decoder consumes an arbitrary finite sequence of
dynamic-table size updates before the first header field. It applies updates
in wire order and decodes the following field using the final accepted
capacity. A table-size update after the first field remains invalid.

Completed HEADERS and `PUSH_PROMISE` blocks and final CONTINUATION assembly use
the same decoder. The receive path checks every leading update against the
active local header-table limit before installing the returned HPACK state.
Malformed, excessive, and misplaced updates retain their focused diagnostics
and report the offending update's absolute HPACK byte offset and inspected
suffix. The standalone production transition also requires the active receive
limit, so no unchecked next state is exposed outside the HTTP/2 wrapper. A
failed block exposes neither a partial header list nor a partially updated
state, so callers can reuse the original input state.

## Evidence

- The production decoder cases cover zero, one, two, and four leading updates
  followed by a header field and verify the final capacity.
- Completed HEADERS, `PUSH_PROMISE`, and final CONTINUATION cases cover the
  shared success path.
- More-than-two leading updates followed separately by malformed, excessive,
  and post-field misplaced updates check the diagnostic family, exact offset,
  inspected suffix, absent success transition, and reusable input state.
- The failure cases exercise the checked standalone transition, completed
  HEADERS and `PUSH_PROMISE`, and final CONTINUATION assembly.

The earlier two-update record remains as history for the bounded fixture
slice. The production sequence policy deliberately removes the update-count
cap; adding another fixed-width branch is not a follow-up target.
