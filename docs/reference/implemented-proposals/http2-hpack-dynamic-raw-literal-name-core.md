# HTTP/2 HPACK Dynamic Raw Literal-Name Core

Status: implemented

This record preserves the completed source-visible raw literal-name receive
slice from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is
specified by `../../specification/execution.md`,
`../../specification/run-json.md`, `../../specification/commands.md`, and the
checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

The `hpack_dynamic_core` boundary accepts the checked HPACK raw literal-name
forms when both the name and value are visible ASCII:

- literal-without-indexing with a raw literal name and raw literal value
- literal-with-indexing with a raw literal name and raw literal value
- literal-never-indexed with a raw literal name and raw literal value

The literal-with-indexing form inserts the decoded name/value pair into the
immutable dynamic-core state using the existing dynamic entry accounting rule.
The literal-without-indexing and literal-never-indexed forms advance the
decode count without mutating the dynamic table. A following `0xbe` dynamic
indexed field can reuse the raw literal-name entry inserted by the
literal-with-indexing form.

Completed HTTP/2 HEADERS and final CONTINUATION paths try the
source-visible raw literal-name boundary before fixture fallback. Accepted
raw literal-name fields update the carried HPACK state through the same
state bridge used by the fixture decoder. Unsupported or malformed raw
literal-name forms still fall back to the existing fixture diagnostics, so
this slice does not add a new focused failure id.

Full HPACK compression, Huffman expansion beyond existing checked support,
production header validation beyond existing rules, outbound behavior, and
unbounded dynamic-table behavior remain outside this narrow receive slice.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  standalone `hpack_dynamic_core` raw literal-name receive for the three
  indexing forms, dynamic-table mutation only for literal-with-indexing, and
  dynamic indexed reuse through `0xbe`.
- Historical aggregate evidence checks completed
  HEADERS and final CONTINUATION routing through the source-visible raw
  literal-name boundary before fixture fallback, including reuse of the
  inserted literal-with-indexing entry.
- `../../specification/execution.md`, `../../specification/run-json.md`, and
  `../../specification/commands.md` summarize the current behavior and route
  readers to the executable evidence.
