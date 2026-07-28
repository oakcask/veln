# Retired HTTP/2 Protocol Core Case

Status: retired

This path is kept only as a route for old documentation links. The broad
`case.toml` and fixture-owned Veln implementation were removed after their
reusable behavior moved to standard-owned modules and focused cases.

This directory contains no reusable Veln implementation. It retains
no migration-only manifests, generated digest tests, or checker-owned
structured projections. Current HTTP/2 behavior is specified by public
standard-library tests and focused executable examples.

Use the focused executable cases whose names start with `http2-core-` for the
current sans-I/O core evidence. Use focused `http2-protocol-core-*` cases for
human and JSON diagnostic projections that still carry the historical case-name
prefix. Current behavior starts at
[http2.md](../../../../docs/specification/http2.md).

The former migration inventory was retired with the broad fixture. Do not add
new evidence under this route. Add current behavior evidence to focused
`http2-core-*`, `http2-protocol-core-*`, or `hpack-*` cases instead.
