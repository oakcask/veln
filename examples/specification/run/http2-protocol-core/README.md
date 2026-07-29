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
  Rows that use `retirement_output_evidence_test.veln` are hashed against the
  named test and the helper bodies reachable from that test, not the whole
  output-evidence file.
- `retirement-scenarios.jsonl`: row-addressable structured projection model.
- `retirement-coverage.tsv`: generated dimension coverage report.
- `retirement_projection_evidence_test.veln`: generated standard-package test
  source that checks bounded projection chunk digests and the aggregate
  projection manifest digest against generated expectation functions.

The structured scenario model keeps stdout input facts as row labels,
projections, frame facts, offsets, streams, sizes, byte counts, and diagnostic
ids. It rejects hash-only stdout inputs and duplicate lifecycle or stream
states. Each row also carries an `executable_projection` recipe that names the
historical row, focused evidence target, public operation to invoke, branch,
initial state, ordered setup, concrete input, result projection, post-state,
output provenance, and diagnostic precedence. The checker rejects
assertion-hash-only recipes and operation substitutions so row evidence cannot
collapse into unrelated assertion bodies. Output stdout rows are routed to the
same output-evidence test that contains the retained output-table assertion
for their table name, and relevance checks inspect only that reachable
assertion source. The generated standard-package test keeps this row set
executable in bounded groups by checking chunk fingerprints, chunk digests,
and the aggregate digest for the same retained rows through generated
expectation functions. The row fingerprints include the terminal
`executable_projection` detail and the full executable-projection recipe code.

Regenerate and verify the structured files and generated test source with
`scripts/check-http2-retirement-evidence --regenerate-structured`, then run
`scripts/check-http2-retirement-evidence`.
