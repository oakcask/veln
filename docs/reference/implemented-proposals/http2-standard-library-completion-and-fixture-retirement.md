# HTTP/2 Standard Library Completion and Fixture Retirement

Status: implemented

Current HTTP/2 behavior is specified by
[`http2.md`](../../specification/http2.md) and checked by focused
standard-package tests and executable specification cases. This record
preserves the completion boundary for retiring the former broad
`http2-protocol-core` fixture and its temporary migration evidence.

## Completion Evidence

Reusable connection, stream, frame, HPACK, continuation, flow-control,
SETTINGS, shutdown, output, receive, and send behavior is owned by
`std::http2::core`, `std::http2::frame`, and `std::http2::hpack`. Production
receive and send paths cross the public HPACK codec rather than fixture
fallback decoding or canned encoding.

The focused standard-package suites cover the public boundaries directly:

- `frame_test.veln` covers frame decode, header encode, and stream-id failure;
- `diagnostic_test.veln` covers stable public protocol diagnostic ids;
- `hpack_test.veln` covers integer and Huffman codecs, static and dynamic
  tables, indexed and literal fields, table-size updates, header-block decode
  and encode, exact bytes, and failure-state preservation;
- `core_test.veln` covers connection and stream state, receive dispatch,
  continuation, flow control, SETTINGS, shutdown, all public send families,
  output ordering, diagnostics, failure atomicity, and decision accessors.

Focused cases under `examples/specification/run/` provide public state, branch,
byte, human diagnostic, and JSON diagnostic projections. Cases whose names
retain the `http2-protocol-core-*` prefix are current focused diagnostic cases,
not the retired broad fixture.

The final migration audit accounted for 652 historical helper invocations,
2,044 stdout projections, and 315 output tables. That inventory was used only
to confirm that current focused evidence owned every implemented behavior. The
historical inventory, structured scenarios, coverage report, generator,
checker, generated projection tests, and retained-output tests were then
removed. Current verification does not read a historical fixture revision or
migration manifest.

## Verification Routes

- `scripts/agent-stdlib-test` runs the public standard-package suites without
  retirement tests.
- `bash scripts/agent-test -p veln-cli --test toolchain_harness` runs the
  executable specification cases.
- `bash scripts/agent-test` runs the workspace gate.

The migration audit is historical rationale only. Later HTTP/2 changes are
validated against the current specification and focused executable evidence.

## Non-Goals

Do not restore reusable behavior, historical row keys, or migration-only
verification authority to the retired broad fixture route.
