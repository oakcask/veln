# HTTP/2 HPACK Static Name Literal Fixture

Status: implemented

This record preserves the completed HPACK static-name literal fixture slice
from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified
by `../../specification/execution.md`, `../../specification/examples.md`, and
the checked executable case
`../../../examples/specification/run/http2-protocol-core/`.

## Completed Behavior

The imported HPACK fixture decoder accepts literal-without-indexing,
literal-with-indexing, and literal-never-indexed header blocks whose
indexed-name form resolves to a supported static-table header name already
accepted by the static-indexed fixture set. That extends the earlier
pseudo-header literal boundary to ordinary names such as `server`,
`content-type`, and `user-agent` while keeping full HPACK compression and a
production dynamic table out of scope.

The literal forms share the existing visible-ASCII string literal decoder and
the existing unsupported-header failure routes for unsupported names,
malformed string lengths, non-visible raw values, and unsupported Huffman
forms. Literal-with-indexing still inserts the decoded name and value into the
bounded immutable fixture dynamic table. Literal-without-indexing and
literal-never-indexed still advance decode state without inserting a dynamic
entry.

## Evidence

- `../../../examples/specification/run/http2-protocol-core/` checks
  literal-without-indexing `server: ok`, literal-with-indexing
  `content-type: text` followed by dynamic-indexed reuse from the inserted
  entry, and literal-never-indexed `user-agent: agent` through a final
  CONTINUATION path.
- `../../specification/execution.md` and `../../specification/examples.md`
  summarize the implemented fixture boundary and route readers to the checked
  example.
