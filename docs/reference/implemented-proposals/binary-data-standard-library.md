# Binary Data Standard Library

Status: implemented

## Outcome

Veln exposes the byte data and immutable buffering vocabulary needed by binary
schemas, codecs, and the checked sans-I/O examples. Current types, pure helper
effects, execution behavior, and diagnostic projection are specified under
`../../specification/types.md`, `../../specification/names-effects.md`,
`../../specification/execution.md`, and `../../specification/run-json.md`.

The implemented boundary includes owned chunks, bounded views, byte counts and
offsets, incremental input, outgoing chunk collections, checked fixed-width
reads and writes in both byte orders, schema conversion, and bounded human and
structured JSON byte previews.

## Evidence

Executable evidence lives under `../../../examples/specification/`, especially
the binary byte-helper, schema conversion, codec, and HTTP/2 diagnostic cases.
Focused completion records in this directory retain the history for individual
slices such as outgoing chunk production, `u56` helpers, schema conversion,
and HPACK byte-preview diagnostics.

## Boundary

This record does not promise a production memory layout, zero-copy behavior,
socket I/O, or an unbounded family of numeric widths or diagnostic variants.
A future byte capability needs a concrete consumer and its own bounded
proposal; extending a width or diagnostic sequence is not remaining work from
this proposal.
