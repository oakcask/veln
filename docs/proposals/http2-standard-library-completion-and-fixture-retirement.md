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
  assertion body, and outbound PUSH_PROMISE state-preservation rows now require
  the retained test body to contain the historical invocation's corresponding
  failure branch projection. Several other distinct helper and stdout shapes
  still map to shared retained tests whose bodies do not mechanically compare
  the item-specific before and after connection value or every nested field
  check.
- Output table evidence still reconstructs many historical byte sequences from
  the retained bytes supplied by the manifest. Empty output tables now require
  the expected production failure id for DATA, WINDOW_UPDATE, PRIORITY,
  HEADERS, and PUSH_PROMISE families, including content-length versus
  flow-control DATA precedence, peer-limit HEADERS precedence,
  SETTINGS_ENABLE_PUSH versus GOAWAY PUSH_PROMISE precedence, and
  GOAWAY-before-output PRIORITY precedence. The HPACK table-size over-limit
  row now checks the legal outbound table-size prefix through the public
  response HEADERS sender and the retained rejected inbound table-size update
  through the public receive and HPACK decoder projections. The
  SETTINGS-ACK-cleared row now starts from an empty outstanding-local-settings
  state before proving that the pending peer ACK is cleared and a second send
  emits no bytes. Successful retained HPACK output rows now cross the
  production decoder and exact public element encoders, so retained successful
  rows no longer depend on a historical-canonical mismatch list. Some empty
  output rows are still grouped by frame family instead of constructing every
  historical starting state and transition before proving the corresponding
  absence of bytes.
- The retirement checker should reject same-domain substitutions that omit the
  historical endpoint role, starting state, diagnostic precedence, complete
  result projection, exact emitted bytes, or failure/no-output state. It now
  rejects same-domain substitutions for outbound PUSH_PROMISE
  state-preservation rows when the retained body omits the branch projection
  selected by the historical invocation. It also rejects reintroducing an
  unrelated local SETTINGS batch into the ACK-cleared row or dropping the
  paired legal-outbound and rejected-inbound table-size projections.

## Implementation Notes

Tighten the retirement checker only when replacement executable evidence is
ready, so the repository does not accept a stricter gate that cannot pass.
Prefer focused standard-package tests and `examples/specification/` cases over
adding new fixture-owned helpers. When a historical row has no practical
mechanical replacement, keep that limitation explicit here instead of marking
the umbrella route implemented.
