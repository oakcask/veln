# HTTP/2 Standard Library Completion and Fixture Retirement

Status: superseded

Current behavior is specified by
[`http2.md`](../../specification/http2.md) and its focused executable routes.
This record preserves historical evidence for migrated slices. Remaining
fixture-retirement work is tracked by
[`http2-standard-library-completion-and-fixture-retirement.md`](../../proposals/http2-standard-library-completion-and-fixture-retirement.md).

Reusable connection, stream, HPACK, receive, send, flow-control,
content-length, shutdown, and output-buffer behavior is owned by
`std::http2::core` and `std::http2::hpack`. Transitions are immutable, failure
decisions preserve caller-owned state and output, and production receive and
send paths use the public HPACK codec.

The broad `http2-protocol-core` implementation and case were removed after
their reusable responsibilities moved to standard-owned modules and focused
cases. Its retained route contains no reusable implementation, but retirement
evidence is not the authority for current behavior.

## Retirement Evidence

This section describes the current retirement-evidence gate. It is not a claim
that every historical row has a complete item-specific replacement for endpoint
role, starting state, diagnostic precedence, emitted bytes, and failure
atomicity.

The checked
[`retirement-evidence.tsv`](../../../examples/specification/run/http2-protocol-core/retirement-evidence.tsv)
manifest contains one row for every historical assertion item:

- 652 `require_*` invocation sites;
- 2,044 exact stdout lines;
- 315 output tables, including each complete table name and chunk list.

Each row retains the complete historical value in base64, the hash of the
retained executable test body or checked case assertion, and an item-specific
hash binding those two values. The
[`check-http2-retirement-evidence`](../../../scripts/check-http2-retirement-evidence)
gate reconstructs the inventory from the parent of the fixture-retirement
change, decodes and compares every complete retained value, and rejects
missing, duplicate, unexpected, changed, or independently rebound rows. A
standard-package reference must reach the public `http2::core` or
`http2::hpack` boundary and check success and failure branches. A focused case
reference must place its evidence needle inside an actual `equals` or
`contains` value; source comments and unrelated case assertions do not count.
Historical output tables and their stdout projections additionally require
the normalized table name and every exact chunk to occur in a checked test
body. References to the focused
[`retirement_output_evidence_test.veln`](../../../crates/veln-stdlib/veln/http2/retirement_output_evidence_test.veln)
must contain one exact call for each historical table rather than a grouped
literal or comment marker. The retained test implementation is part of each
binding hash. Complete frame sequences are decoded and reconstructed one frame
at a time through the public frame codec; non-frame vectors cross the
production HPACK decoder. Successful decodes must consume the complete
retained vector and send the decoded header list back through the public
production HPACK encoder, while rejected decodes must expose the expected
production failure kind for the retained vector without changing the
caller-owned dynamic table. Singleton zero-length vectors exercise their
historical WINDOW_UPDATE or HEADERS send rejection and verify both decision
bytes and the output buffer remain empty. Empty tables are classified by
historical frame domain and retained table-name family, then exercise a
production send failure before the same failure-atomicity checks. Repeated
chunks therefore cannot satisfy multiple table rows through one occurrence,
empty rows cannot be satisfied by
an unclassified same-kind rejection, and complete frames or HPACK vectors
cannot be accepted as nested DATA payloads, by failed-decode input identity, or
by a generic non-empty HPACK failure check.

The checker derives a required public protocol domain for every helper
invocation from its helper and caller, including component-specific
connection-state preservation checks. Outbound HEADERS and PUSH_PROMISE helper
evidence distinguishes accepted transitions, rejected transitions, and
failure-atomic state preservation so a same-domain success test cannot satisfy
a historical failure helper. The three connection-stream helper sites must
reference SETTINGS, PING, and GOAWAY tests respectively.
Production HTTP/2 diagnostic stdout rows must compare their retained
diagnostic id. The historical
`:fixture` continuation projection is bound to a production receive-frame
test that checks a split CONTINUATION, a nine-octet HPACK block, one resulting
stream, and immutable input state. Generic stdout projections additionally
require a matching SETTINGS, HPACK, receive-frame, send-transition,
flow-control, content-length, stream-collection, priority, output-encoding, or
shutdown boundary according to the retained projection kind. Unclassified
helper and stdout projection kinds fail the gate. The checker self-test
rejects the former peer-stream, outbound HEADERS success-for-failure,
unrelated chunk, generic HPACK, generic non-empty HPACK failure, nested-DATA,
grouped-output, comment-only empty-output, inbound-entry, and encode-error
substitutions.

The historical peer-stream failure helper's component checks are split across
the retained connection composition, stream collection, flow-control,
content-length, priority, HPACK, and shutdown tests. They are not all assigned
to the narrower peer-stream-admission assertion.

The focused standard-package tests and executable specification cases remain
independently runnable without the historical fixture. They cover migrated
state transitions and diagnostic projections through the public
standard-library boundary, while the remaining item-specific retirement gaps
stay tracked as planned work. Explicitly selecting
[`retirement_output_evidence_test.veln`](../../../crates/veln-stdlib/veln/http2/retirement_output_evidence_test.veln)
keeps the complete standard-package analysis closure while selecting only that
file's tests from the standard-package root. Standard-package test execution
generates the shared JVM program once and dispatches each selected test by
name, so the full guarded suite does not regenerate the complete class set for
every test.
