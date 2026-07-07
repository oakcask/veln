# HTTP/2 HPACK Dynamic-Name Literal Core

Status: implemented

This record preserves the completed source-visible dynamic-name literal
receive slice from the HTTP/2 sans-I/O protocol-core proposal. Current
behavior is specified by `../../specification/execution.md`,
`../../specification/run-json.md`, and the checked executable case
`../../../examples/specification/run/hpack-fixture-codec-boundary/`.

## Completed Behavior

The `hpack_dynamic_core` boundary accepts checked HPACK literal header fields
whose header name is resolved from the caller-supplied bounded dynamic table:

- literal-without-indexing with a dynamic-table name and raw literal value
- literal-with-indexing with a dynamic-table name and raw literal value
- literal-never-indexed with a dynamic-table name and raw literal value

The literal-with-indexing form inserts the decoded name/value pair into the
immutable dynamic-core state using the existing dynamic-entry accounting rule.
The literal-without-indexing and literal-never-indexed forms advance the
decode count without mutating the dynamic table. A following `0xbe` dynamic
indexed field can reuse the entry inserted by the literal-with-indexing form,
and `0xbf` can still resolve the older retained entry when the bounded table
has room.

This remains a bounded source-visible receive core. It does not add full HPACK
compression, unbounded dynamic-table behavior, new static-table entries, new
HTTP/2 frame-state behavior, or production header validation beyond existing
rules.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  standalone `hpack_dynamic_core` dynamic-name receive for the three indexing
  forms, dynamic-table mutation only for literal-with-indexing, retained-entry
  reuse for non-inserting forms, and inserted-entry reuse through `0xbe`.
- The same case checks retained older-entry reuse through `0xbf` after the
  literal-with-indexing form inserts a replacement value.
- `../../specification/execution.md` and `../../specification/run-json.md`
  summarize the current behavior and route readers to the executable evidence.
