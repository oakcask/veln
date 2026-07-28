# HTTP/2 Standard Library Completion and Fixture Retirement

Status: implemented

Current HTTP/2 behavior starts in
[`../../specification/http2.md`](../../specification/http2.md). This record
preserves the completion boundary for the retired broad
`http2-protocol-core` route.

## Completion Boundary

Reusable connection and stream behavior is owned by `std::http2::core`.
Production receive and send paths use the public `std::http2::hpack` codec for
header-block decoding and encoding. The retired broad route is not an
executable case; focused `http2-core-*` and `http2-protocol-core-*` examples
carry current observable behavior.

Each historical `require_*` invocation, exact stdout line, and
`output_chunk_list` assertion from the retired route is bound to retained
mechanical evidence by `scripts/check-http2-retirement-evidence`. The checker
requires the historical value, retained assertion body, and combined binding to
match, and it rejects unclassified evidence rows. Its relevance checks reject
same-domain substitutions that omit the retained endpoint role, starting state,
diagnostic id, result projection, emitted bytes, or failure-atomicity
projection.

## Evidence

- `crates/veln-stdlib/veln/http2/core_test.veln` checks reusable connection,
  stream, HPACK, continuation, flow-control, output-buffer, and failure
  atomicity behavior through the standard package.
- `crates/veln-stdlib/veln/http2/hpack_test.veln` checks the public production
  HPACK codec.
- `crates/veln-stdlib/veln/http2/retirement_output_evidence_test.veln` checks
  retained output rows through production frame, HPACK, send, response, and
  output-buffer boundaries. Empty DATA rows bind each retained name to its own
  starting stream entry, payload, end-stream flag, and exact content-length or
  flow-control failure projection.
- `examples/specification/run/http2-core-*` and
  `examples/specification/run/http2-protocol-core-*` cases retain the
  observable command, human diagnostic, JSON diagnostic, and emitted-byte
  evidence routed from the HTTP/2 specification.

## Verification

The independent retirement checker reports complete retained coverage:

- helper invocations: 652
- stdout lines: 2044
- output tables: 315
- unclassified items: 0

The checker self-test covers the former broad-substitution risks, including
accepted outbound HEADERS evidence for rejected helper rows, inbound
PUSH_PROMISE evidence for outbound state preservation, unrelated stdout
projections, grouped output evidence, nested DATA output evidence, failed HPACK
decode input-identity evidence, generic non-empty HPACK failures, and stale
empty-output substitutions.

The guarded standard-package run passes the retained HTTP/2 output evidence
tests together with the focused HTTP/2 core and HPACK tests.
