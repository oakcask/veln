# HTTP/2 Standard Library Completion and Fixture Retirement

Status: proposed

## Goal

Complete the remaining HTTP/2 sans-I/O core ownership and evidence
classification that was left open when the broad
`examples/specification/run/http2-protocol-core/` fixture was retired.
Production HPACK encoding and decoding are already implemented through
`http2::hpack`; do not add another HPACK representation or compatibility
fallback.

## Current Implemented Boundary

`docs/specification/http2.md` is the source of current behavior. It already
specifies the public `http2::core` component state, complete-frame dispatcher,
SETTINGS, DATA, HEADERS, CONTINUATION, PUSH_PROMISE, WINDOW_UPDATE,
RST_STREAM, GOAWAY, outbound DATA, outbound WINDOW_UPDATE, outbound HEADERS,
outbound PUSH_PROMISE, outbound RST_STREAM, outbound PRIORITY, local SETTINGS,
PING helpers, shutdown helpers, output-buffer helpers, and focused executable
evidence. It also specifies the immutable chunked receive boundary that
composes server preface consumption, the initial peer SETTINGS gate, buffered
single-frame extraction, `apply_receive_frame(...)`, inbound SETTINGS ACK
output, inbound PING ACK output, and inbound PRIORITY offset application.

Those completed slices must be reused. The remaining proposal work is not a
new HPACK codec, another repeated frame slice, or another domain-only helper.

## Remaining Implementation Scope

- Extend the standard-owned chunked receive boundary from single-frame
  extraction to the remaining ordered receive-loop behavior needed by the
  retired aggregate matrix.
- Preserve diagnostic precedence and failure atomicity across input,
  connection, stream, HPACK, continuation, flow-control, content-length,
  shutdown, and output-byte state for the remaining matrix rows.
- Keep the public production `http2::hpack` typed codec as the only reachable
  HPACK boundary for protocol-core receive and send transitions.

## Evidence Scope

The retired aggregate fixture deleted evidence before every assertion was
classified. Reconstruct the checked matrix from the parent of the retirement
change and finish it before treating the proposal as complete:

- classify every removed `require_*` helper invocation;
- classify every removed exact-stdout line;
- classify all removed `output_chunk_list` tables;
- classify every focused `HpackFixtureState`, `hpack_fixture::`, or
  `hpack.fixture.unsupported_header_block` marker as either intentional
  diagnostic evidence or residual compatibility behavior; and
- add adjacent standard tests and focused executable cases for missing
  ordered receive, PRIORITY, PING, diagnostic, result, and emitted-byte
  behavior.

## Documentation Scope

After executable evidence exists, reconcile `docs/specification/http2.md` with
the new ordered receive behavior and the already implemented PUSH_PROMISE
dispatcher. The proposal can move back to
`docs/reference/implemented-proposals/` only after the receive boundary,
inbound transitions, and checked evidence matrix are complete.

## Non-Goals

- No TLS, ALPN, sockets, adapter ownership, or unrelated throughput work.
- No compatibility aliases for the retired aggregate fixture.
- No weakened diagnostics or relaxed failure-state preservation.
- No proposal completion claim based only on fixture deletion.
