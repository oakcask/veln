# Retired HTTP/2 Protocol Core Case

Status: retired

This path is kept only as a route for old documentation links. The broad
`case.toml` and fixture-owned Veln implementation were removed after their
reusable behavior moved to standard-owned modules and focused cases.

This directory contains no reusable Veln implementation. It retains
migration-only evidence for the completed semantic retirement gate; that
evidence is not current HTTP/2 behavior.

Use the focused executable cases whose names start with `http2-core-` for the
current sans-I/O core evidence. Use focused `http2-protocol-core-*` cases for
human and JSON diagnostic projections that still carry the historical case-name
prefix. Current behavior starts at
[http2.md](../../../../docs/specification/http2.md).

The checked migration artifacts are:

- `retirement-evidence.tsv`: historical row binding to executable evidence.
- `retirement-scenarios.jsonl`: row-addressable structured projection model.
- `retirement-coverage.tsv`: generated dimension coverage report.

The structured scenario model keeps stdout input facts as row labels,
projections, frame facts, offsets, streams, sizes, byte counts, and diagnostic
ids. It rejects hash-only stdout inputs and duplicate lifecycle or stream
states so row evidence cannot collapse into shared assertion bodies.

Regenerate and verify the structured files with
`scripts/check-http2-retirement-evidence --regenerate-structured`, then run
`scripts/check-http2-retirement-evidence`.
