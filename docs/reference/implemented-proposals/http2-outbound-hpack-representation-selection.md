# HTTP/2 Outbound HPACK Representation Selection

Status: implemented

This record preserves the completed bounded representation-selection slice
from the HTTP/2 sans-I/O protocol-core proposal. Current behavior is specified
by `../../specification/execution.md`, `../../specification/run-json.md`, and
the checked executable cases under `../../../examples/specification/run/`.

## Completed Behavior

The source-visible selector accepts an ordered two-header list, immutable
carried HPACK fixture state, an active dynamic-table capacity, and an input
offset. It processes headers in order with this deterministic precedence:
exact static indexed, exact dynamic indexed, static-name literal,
dynamic-name literal, then new-name literal.

All selected literals use the existing raw literal-with-indexing policy.
Literal selections insert through the bounded dynamic-table helper; exact
indexed selections preserve the table. The active capacity must match the
capacity carried by the state. Insertions use that capacity and retain or
evict entries through the existing table accounting rules.

The checked seed block selects static indexed `:method: GET` before inserting
new-name `x-trace: ok`. A second block selects the carried exact dynamic entry
before inserting `x-trace: again` through its dynamic name. A separate block
checks static-name literal selection followed by exact static selection.
Reduced-capacity coverage proves that an oversized second insertion clears the
bounded table. Invalid names and mismatched capacity remain focused fixture
failures, and the original carried state remains reusable after either
failure.

The outbound HEADERS header-list path tries the selector before its existing
fixture encoder fallback. Both mixed blocks enter that path using the
selector result and returned state. This slice adds no compression-cost
heuristic, automatic Huffman policy, representation family, unbounded list or
table behavior, inbound behavior, or transport behavior.

## Evidence

- `../../../examples/specification/run/hpack-fixture-codec-boundary/` checks
  representation precedence, exact bytes, carried state, reduced-capacity
  eviction, focused failures, and state reuse after failure.
- `../../../examples/specification/run/http2-protocol-core/` routes the seed
  and carried-state blocks through outbound HEADERS and checks exact frames.
- `../../specification/execution.md` and `../../specification/run-json.md`
  summarize current behavior and route readers to executable evidence.
