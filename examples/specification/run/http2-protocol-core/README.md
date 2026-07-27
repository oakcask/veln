# Retired HTTP/2 Protocol Core Case

Status: retired

This path is kept only as a route for old documentation links. The broad
`case.toml` and fixture-owned Veln implementation were removed, but that
retirement is not itself proof that every reusable HTTP/2 behavior moved
behind `http2::core` and `http2::hpack` in the toolchain-owned `std` package.

Use the focused executable cases whose names start with `http2-core-` for the
current sans-I/O core evidence. Use focused `http2-protocol-core-*` cases for
human and JSON diagnostic projections that still carry the historical case-name
prefix. The remaining ordered receive and evidence-classification work is
routed from
[http2-sans-io-protocol-core.md](../../../../docs/proposals/http2-sans-io-protocol-core.md).
