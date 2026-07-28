# Retired HTTP/2 Protocol Core Case

Status: retired

This path is kept only as a route for old documentation links. The broad
`case.toml` and fixture-owned Veln implementation were removed after their
reusable behavior moved to standard-owned modules and focused cases.

[`retirement-evidence.tsv`](retirement-evidence.tsv) maps every historical
helper invocation, exact stdout line, and output table to a retained
executable assertion. Each row retains the complete historical value as
base64, not only its digest. The retirement checker decodes and compares that
value with the historical fixture and binds it to the focused assertion body.
Those bindings currently establish inventory integrity and broad protocol
relevance, not complete item-specific semantic equivalence. The focused
retirement-output test uses one exact executable call per retained table, and
its shared implementation is included in the binding hash. It checks frame and
HPACK codec reconstruction and selected production send and response failures.
The checker rejects grouped literals, comment-only labels, nested DATA
substitutions, and several generic HPACK and send-failure substitutions.

Most non-empty rows still derive their observed codec value from the retained
bytes rather than the owning production transition. Several empty rows also
substitute historical setup state, input sequences, or complete result
projections. The active proposal defines the structured scenarios and
production-derived output required to close those gaps.

The checker derives a public protocol domain for every helper invocation and
stdout projection. It has stronger branch and diagnostic checks for selected
connection-stream, outbound HEADERS, PUSH_PROMISE, and continuation evidence.
Other relevance checks remain domain-oriented and may accept a same-domain
test that omits concrete historical arguments or state projections.
Veln references must exercise the public HTTP/2 standard-library boundary
through a checked branch. Case references must place their evidence needle
inside an `equals` or `contains` assertion. This
directory contains no reusable Veln implementation. The retained manifest is a
compatibility and audit route, not the source of current HTTP/2 behavior.

From the standard-package root, the retained output gate is independently
runnable with:

```text
veln test http2/retirement_output_evidence_test.veln
```

Use the focused executable cases whose names start with `http2-core-` for the
current sans-I/O core evidence. Use focused `http2-protocol-core-*` cases for
human and JSON diagnostic projections that still carry the historical case-name
prefix. Current behavior starts at
[http2.md](../../../../docs/specification/http2.md). Remaining item-specific
fixture-retirement evidence is tracked by
[http2-standard-library-completion-and-fixture-retirement.md](../../../../docs/proposals/http2-standard-library-completion-and-fixture-retirement.md),
with migrated historical slices preserved by
[http2-sans-io-protocol-core.md](../../../../docs/reference/implemented-proposals/http2-sans-io-protocol-core.md).
