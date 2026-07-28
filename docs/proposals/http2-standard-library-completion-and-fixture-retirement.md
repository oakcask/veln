# HTTP/2 Standard Library Completion and Fixture Retirement

Status: proposed

This proposal tracks the remaining HTTP/2 sans-I/O core and fixture-retirement
evidence work. Current implemented behavior starts in
[`../specification/http2.md`](../specification/http2.md). Completed historical
slices remain under
[`../reference/implemented-proposals/`](../reference/implemented-proposals/).

## Goal

Complete the remaining HTTP/2 sans-I/O core and evidence migration so the
retired broad `http2-protocol-core` route has mechanically checked replacement
coverage for each historical assertion item. Production HPACK encoding and
decoding remain the only production codec boundary; no duplicate fixture codec
or sequence-extension slice is part of this work.

## Completion Criteria

- Reusable connection and stream behavior is owned by `std::http2::core`, with
  immutable transitions and failure-state preservation across connection,
  stream, HPACK, continuation, flow-control, and output state.
- Production receive and send paths use the public `std::http2::hpack` codec
  rather than fixture state, fallback decoding, canned encoding, or
  `hpack.fixture.unsupported_header_block` compatibility.
- Every retired `require_*` invocation, exact stdout line, and
  `output_chunk_list` assertion has mechanically checked replacement evidence
  preserving the relevant endpoint role, starting state, diagnostic
  precedence, result projection, emitted bytes, and failure atomicity.
- The broad fixture has no reusable implementation or unclassified assertion
  remaining, and replacement coverage passes independently.
- Observable behavior is present in executable specification evidence and
  `../specification/http2.md`; completed proposal records and catalogs do not
  describe unfinished work or route current evidence through retired files.
- The guarded standard-package, focused protocol, loader, performance, and
  workspace verification gates pass without relaxed limits or material
  regression.

## Remaining Gaps

- Helper and stdout relevance is still too broad for some historical items.
  The checker binds each row to a complete historical value and retained
  assertion body, but several distinct helper shapes still map to shared
  retained tests whose bodies do not mechanically compare the item-specific
  before and after connection value or every nested field check.
- Output table evidence still reconstructs many historical byte sequences from
  the retained bytes supplied by the manifest. Empty output tables now require
  the expected production failure id for DATA, WINDOW_UPDATE, and PRIORITY
  families, including content-length versus flow-control DATA precedence and
  GOAWAY-before-output PRIORITY precedence. Some empty output rows are still
  grouped by frame family instead of constructing every historical starting
  state and transition before proving the corresponding absence of bytes.
- The retirement checker should reject same-domain substitutions that omit the
  historical endpoint role, starting state, diagnostic precedence, complete
  result projection, exact emitted bytes, or failure/no-output state.

## Implementation Notes

Tighten the retirement checker only when replacement executable evidence is
ready, so the repository does not accept a stricter gate that cannot pass.
Prefer focused standard-package tests and `examples/specification/` cases over
adding new fixture-owned helpers. When a historical row has no practical
mechanical replacement, keep that limitation explicit here instead of marking
the umbrella route implemented.
