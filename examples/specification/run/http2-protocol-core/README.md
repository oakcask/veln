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
checks their item-specific binding. It also requires output-table names and
octets to occur in executable comparisons, binds connection-stream helpers to
their SETTINGS, PING, or GOAWAY domain, checks retained production diagnostic
ids, and rejects a continuation projection unless the referenced test checks
its continuation, wire-size, and single-decode result. Veln references must
exercise the public HTTP/2 standard-library boundary through a checked branch.
Case references must place their evidence needle inside an `equals` or
`contains` assertion. This directory contains no reusable Veln implementation.

Use the focused executable cases whose names start with `http2-core-` for the
current sans-I/O core evidence. Use focused `http2-protocol-core-*` cases for
human and JSON diagnostic projections that still carry the historical case-name
prefix. Current behavior starts at
[http2.md](../../../../docs/specification/http2.md). Completion evidence is
recorded by
[http2-sans-io-protocol-core.md](../../../../docs/reference/implemented-proposals/http2-sans-io-protocol-core.md).
