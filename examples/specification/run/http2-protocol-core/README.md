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
- `retirement_projection_evidence_test.veln`: generated standard-package test
  source that executes row-addressed projection recipe and SHA checks plus the
  aggregate projection manifest digest against generated expectation
  functions.

The structured scenario model keeps stdout input facts as row labels,
projections, frame facts, offsets, streams, sizes, byte counts, and diagnostic
ids. It rejects hash-only stdout inputs and duplicate lifecycle or stream
states. Each row also carries an `executable_projection` recipe that names the
historical row, focused evidence target, public operation to invoke, branch,
initial state, ordered setup, concrete input, result projection, post-state,
output provenance, and diagnostic precedence. The checker rejects
assertion-hash-only recipes and operation substitutions so row evidence cannot
collapse into shared assertion bodies. The generated standard-package test
executes every row id with its observed projection recipe and SHA in bounded
row groups, compares both row values with generated expectation functions,
then checks the aggregate digest for the same retained row set through the
manifest expectation function.

Regenerate and verify the structured files and generated test source with
`scripts/check-http2-retirement-evidence --regenerate-structured`, then run
`scripts/check-http2-retirement-evidence`.
