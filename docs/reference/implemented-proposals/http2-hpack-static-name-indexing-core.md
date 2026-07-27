# HTTP/2 HPACK Static-Name Indexing Core

Status: implemented

This record preserves the completed source-visible static-name
literal-with-indexing receive slice from the HTTP/2 sans-I/O protocol-core
proposal. Current behavior is specified by
`../../specification/execution.md`, `../../specification/run-json.md`,
`../../specification/commands.md`, and the checked executable cases under
`../../../examples/specification/run/`.

## Completed Behavior

The `hpack_dynamic_core` boundary accepts the checked static-name
literal-with-indexing representation when the field name resolves through the
HPACK static table and the value is a bounded raw visible-ASCII string. The
checked `content-type: text` block returns the decoded header entry and wire
size, inserts the decoded name/value pair into immutable dynamic-core state
using the existing dynamic-entry accounting rule, and resolves a following
`0xbe` dynamic indexed field from the inserted entry.

Completed HTTP/2 HEADERS and final CONTINUATION paths try the source-visible
static decoder before fixture fallback for supported static-name
literal-with-indexing blocks. When the static decoder owns the block, the
HTTP/2 decode state inserts the decoded entry into the carried HPACK dynamic
state, so a later `0xbe` dynamic indexed field can reuse that entry without
decoding the literal through the broad fixture fallback.

Unsupported static indexes, malformed string lengths, malformed raw strings,
malformed Huffman padding, Huffman EOS, dynamic table lookup boundaries,
dynamic-name continuations, table-size updates, literal-name forms, full HPACK
compression, and unbounded dynamic-table behavior remain outside this narrow
source-visible receive slice.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  source-visible static-name literal-with-indexing decode for
  `content-type: text`, immutable dynamic-core insertion with accounting, and
  dynamic indexed reuse through `0xbe`.
- Historical aggregate evidence checks completed
  HEADERS and final CONTINUATION routing through the source-visible static
  decoder while preserving carried HPACK dynamic state and existing focused
  fixture diagnostics for unsupported forms.
- `../../specification/execution.md`, `../../specification/run-json.md`, and
  `../../specification/commands.md` summarize the current behavior and route
  readers to the executable evidence.
