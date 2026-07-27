# HTTP/2 Standard Library Completion and Fixture Retirement

Status: implemented

Current behavior is specified by
[`http2.md`](../../specification/http2.md) and its focused executable routes.
This record preserves the completion boundary and verification evidence.

## Completed Boundary

The standard `http2::core` and `http2::hpack` implementations own immutable
connection and stream state, production HPACK, receive and send transitions,
flow control, header and content-length validation, shutdown, and output
buffering. The former aggregate `http2-protocol-core` implementation is
retired; its remaining directory is only a historical route.

The retired assertion surface is preserved by two checked records:

- [aggregate assertion classification](http2-sans-io-aggregate-assertion-classification.md)
  covers 652 helper invocations, 2,044 exact stdout lines, and 315 output
  tables;
- [fixture marker classification](http2-sans-io-fixture-marker-classification.md)
  covers the focused examples that intentionally retain historical fixture
  diagnostics.

`../../../scripts/check-http2-retirement-evidence` reconstructs the aggregate
inventory from its historical source revision, validates every item against
an existing exact standard test or focused assertion, and reports zero
unclassified items. `--inventory` emits the complete item-level mapping.

## Verification

- The guarded standard package test passes all 207 tests in 203.23 seconds
  with a maximum resident set size of 295,624 KiB.
- The complete toolchain harness passes all 1,327 executable specification
  cases, including the focused HTTP/2 protocol cases.
- The guarded workspace test suite, workspace check, standard-library loader
  tests, evidence checker, formatting, and Markdown target checks pass.

The standard test speedup reuses the already validated full typed IR only for
the toolchain-owned standard project. Application projects retain per-entry
reachability lowering so unreachable or standard-body behavior does not alter
test execution semantics.
