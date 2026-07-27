# HTTP/2 Standard Library Completion and Fixture Retirement

Status: implemented

Current behavior is specified by
[`http2.md`](../../specification/http2.md) and its focused executable routes.
This record preserves the completion and fixture-retirement evidence.

Reusable connection, stream, HPACK, receive, send, flow-control,
content-length, shutdown, and output-buffer behavior is owned by
`std::http2::core` and `std::http2::hpack`. Transitions are immutable, failure
decisions preserve caller-owned state and output, and production receive and
send paths use the public HPACK codec.

The broad `http2-protocol-core` implementation and case were removed after
their reusable responsibilities moved to standard-owned modules and focused
cases. Its retained route contains no reusable implementation.

## Retirement Evidence

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

The historical peer-stream failure helper's component checks are split across
the retained connection composition, stream collection, flow-control,
content-length, priority, HPACK, and shutdown tests. They are not all assigned
to the narrower peer-stream-admission assertion.

The focused standard-package tests and executable specification cases remain
independently runnable without the historical fixture. They cover endpoint
roles, starting state, diagnostic precedence, result projections, exact
emitted bytes, and failure atomicity through the public standard-library
boundary.
