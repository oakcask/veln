# Retired HTTP/2 Protocol Core Case

Status: retired

This path is kept only as a route for old documentation links. The broad
`case.toml` and fixture-owned Veln implementation were removed after their
reusable behavior moved to standard-owned modules and focused cases.

[`retirement-evidence.tsv`](retirement-evidence.tsv) maps every historical
helper invocation, exact stdout line, and output table to a retained
executable assertion. The retirement checker verifies both the complete
historical value hash and the retained assertion body hash; this directory
contains no reusable Veln implementation.

Use the focused executable cases whose names start with `http2-core-` for the
current sans-I/O core evidence. Use focused `http2-protocol-core-*` cases for
human and JSON diagnostic projections that still carry the historical case-name
prefix. Current behavior starts at
[http2.md](../../../../docs/specification/http2.md). Completion evidence is
recorded by
[http2-sans-io-protocol-core.md](../../../../docs/reference/implemented-proposals/http2-sans-io-protocol-core.md).
