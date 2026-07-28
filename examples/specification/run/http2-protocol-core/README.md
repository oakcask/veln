# Retired HTTP/2 Protocol Core Case

Status: retired

This path is kept only as a route for old documentation links. The broad
`case.toml` and fixture-owned Veln implementation were removed after their
reusable behavior moved to standard-owned modules and focused cases.

This directory contains no reusable Veln implementation. It retains the
historical retirement inventory for the open semantic evidence gate; that
inventory is migration evidence, not current HTTP/2 behavior.

Use the focused executable cases whose names start with `http2-core-` for the
current sans-I/O core evidence. Use focused `http2-protocol-core-*` cases for
human and JSON diagnostic projections that still carry the historical case-name
prefix. Current behavior starts at
[http2.md](../../../../docs/specification/http2.md). Migration evidence is
tracked by the active
[HTTP/2 retirement evidence proposal](../../../../docs/proposals/http2-standard-library-completion-and-fixture-retirement.md).
