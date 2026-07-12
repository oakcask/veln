# HTTP/2 Outbound HPACK Dynamic-Name Huffman Values

Status: implemented

This record preserves the completed outbound dynamic-name Huffman-value slice
from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified
by `../../specification/execution.md`, `../../specification/run-json.md`, and
the checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

The source-visible HPACK fixture encoder accepts literal-without-indexing,
literal-with-indexing, and literal-never-indexed fields whose name resolves
through the carried bounded dynamic table and whose value is encoded by the
existing checked Huffman encoder. With `:path` at dynamic index 62, the checked
`test` value emits `0x0f 0x2f 0x83 0x49 0x50 0x9f`,
`0x7e 0x83 0x49 0x50 0x9f`, and
`0x1f 0x2f 0x83 0x49 0x50 0x9f` for the three forms.

Literal-with-indexing inserts `:path: test` as the newest bounded entry, so a
later header list reuses it as `0xbe` while the older `:path: /target` remains
reachable as `0xbf`. Literal-without-indexing and literal-never-indexed do not
insert a replacement and keep `:path: /target` reusable as `0xbe`.

Missing dynamic-name state and unsupported source input remain focused HPACK
fixture encode failures. Neither failure returns a successful header block or
mutates the carried state. Supported results for all three forms enter the
existing outbound HEADERS framing boundary without adding source syntax or
transport behavior.

Full HPACK compression, unbounded dynamic tables, static-name literals, raw
value literals, inbound decoding, frame splitting changes, and new Huffman
vocabulary remain outside this completed slice.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  exact bytes for all three forms, insertion and later indexed reuse, retained
  state for non-inserting forms, focused missing-name and unsupported-value
  failures, and retained reuse after failures.
- `../../../examples/specification/run/http2-protocol-core/` routes all three
  supported blocks through outbound HEADERS, checks the emitted frame bytes,
  and reuses the Huffman-valued indexed entry in a later HEADERS block.
- `../../specification/execution.md` and `../../specification/run-json.md`
  summarize current behavior and route readers to the executable evidence.
