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

Each row fixes the complete historical value hash and the hash of the retained
executable test body or checked case assertion. The
[`check-http2-retirement-evidence`](../../../scripts/check-http2-retirement-evidence)
gate reconstructs the inventory from the parent of the fixture-retirement
change, rejects missing, duplicate, unexpected, or changed rows, requires
executable test bodies to contain checked success and failure paths, and
requires focused example references to include exact output assertions.
Consequently, caller names, helper names, diagnostic ids, and table-name
prefixes alone cannot satisfy the gate.

The focused standard-package tests and executable specification cases remain
independently runnable without the historical fixture. They cover endpoint
roles, starting state, diagnostic precedence, result projections, exact
emitted bytes, and failure atomicity through the public standard-library
boundary.
