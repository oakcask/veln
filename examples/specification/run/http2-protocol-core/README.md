# Retired HTTP/2 Protocol Core Case

Status: retired

This path is kept only as a route for old documentation links. The broad
`case.toml` and fixture-owned Veln implementation were removed after their
reusable behavior moved to standard-owned modules and focused cases.

This directory contains no reusable Veln implementation, retirement manifest,
or migration-only checker input. Historical fixture-retirement records live
under implemented proposal history, while current behavior is verified by
focused executable cases and standard-package tests.

Use the focused executable cases whose names start with `http2-core-` for the
current sans-I/O core evidence. Use focused `http2-protocol-core-*` cases for
human and JSON diagnostic projections that still carry the historical case-name
prefix. Current behavior starts at
[http2.md](../../../../docs/specification/http2.md). Migration history is
preserved by
[http2-sans-io-protocol-core.md](../../../../docs/reference/implemented-proposals/http2-sans-io-protocol-core.md).
