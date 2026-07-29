# Retired HTTP/2 Protocol Core Case

Status: retired

This path is kept only as a route for old documentation links. The broad
`case.toml` and fixture-owned Veln implementation were removed after their
reusable behavior moved to standard-owned modules and focused cases.

This directory contains no reusable Veln implementation. It retains
no migration-only evidence; current behavior lives in focused standard-module
tests and executable specification cases.

Use the focused executable cases whose names start with `http2-core-` for the
current sans-I/O core evidence. Use focused `http2-protocol-core-*` cases for
human and JSON diagnostic projections that still carry the historical case-name
prefix. Current behavior starts at
[http2.md](../../../../docs/specification/http2.md). The former
retirement manifests, generated row tests, and checker were removed after the
replacement coverage became independently executable through those current
routes.
