# Retired HTTP/2 Protocol Core Case

Status: retired

This path is kept only as a route for old documentation links. The broad
`case.toml` and fixture-owned Veln implementation were removed after their
reusable behavior moved to standard-owned modules and focused cases.

[`retirement-evidence.tsv`](retirement-evidence.tsv) maps every historical
helper invocation, exact stdout line, and output table to a retained
executable assertion. Each row retains the complete historical value as
base64, not only its digest. The retirement checker decodes and compares that
value with the historical fixture, checks the focused assertion body, and
checks their item-specific binding. The focused retirement-output test uses one
exact executable call per retained singleton or empty table, so grouped
literals, comment-only table labels, and one occurrence reused for duplicate
chunks do not count. The shared retained implementation is included in the
binding hash. It reconstructs complete frame sequences with the public frame
codec and sends non-frame vectors through the production HPACK decoder.
Successful HPACK receives must consume the full vector and re-encode each
indexed field, literal field, and table-size update through the public
production HPACK element encoders to the exact retained bytes; failed receives
must report the expected production failure kind for the retained vector while
preserving the caller-owned table.
Singleton zero-length chunks exercise the corresponding rejected
WINDOW_UPDATE or HEADERS send, including unchanged output. Empty output is
classified by historical frame domain and retained table-name family. Rejected
send rows must observe the expected production send failure id before checking
that no decision bytes or output-buffer chunks were emitted. DATA empty-output
rows distinguish content-length failures from flow-control failures, and
PRIORITY empty-output rows construct an open stream in a draining connection so
the GOAWAY boundary wins with the retained diagnostic precedence. The retained
PING-ACK-received row exercises the public PING response no-output transition,
not an outbound PING length failure. The checker rejects nested-DATA,
failed-decode input-identity
substitutions, canonical-equivalent HPACK encoding fallbacks, generic
non-empty HPACK failure checks, and generic data, window, or priority
failure-id checks.

The checker also derives a public protocol domain for every helper invocation,
binds connection-stream helpers to their SETTINGS, PING, or GOAWAY domain,
and separates outbound HEADERS and PUSH_PROMISE accepted, rejected, and
state-preservation evidence. A retained failure helper therefore cannot be
satisfied by an accepted same-domain send test. The checker requires every
stdout projection kind to reach the matching public protocol boundary, checks
retained production diagnostic ids, and rejects a continuation projection
unless the referenced test checks its continuation, wire-size, and
single-decode result. Veln references must exercise the public HTTP/2
standard-library boundary through a checked branch. Case references must place
their evidence needle inside an `equals` or `contains` assertion. This
directory contains no reusable Veln implementation. The retained manifest is a
compatibility and audit route, not the source of current HTTP/2 behavior and
not a claim that every historical row has complete item-specific replacement
coverage.

From the standard-package root, the retained output gate is independently
runnable with:

```text
veln test http2/retirement_output_evidence_test.veln
```

Use the focused executable cases whose names start with `http2-core-` for the
current sans-I/O core evidence. Use focused `http2-protocol-core-*` cases for
human and JSON diagnostic projections that still carry the historical case-name
prefix. Current behavior starts at
[http2.md](../../../../docs/specification/http2.md). Remaining planned
fixture-retirement work is tracked by
[http2-standard-library-completion-and-fixture-retirement.md](../../../../docs/proposals/http2-standard-library-completion-and-fixture-retirement.md),
with migrated historical slices preserved by
[http2-sans-io-protocol-core.md](../../../../docs/reference/implemented-proposals/http2-sans-io-protocol-core.md).
