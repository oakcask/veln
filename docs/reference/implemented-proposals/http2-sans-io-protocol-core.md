# HTTP/2 Sans-I/O Protocol Core

Status: implemented

Current behavior is specified by
[`http2.md`](../../specification/http2.md) and its focused executable routes.
This record preserves the migration history for the former broad
`http2-protocol-core` fixture.

Reusable connection, stream, HPACK, receive, send, flow-control,
content-length, shutdown, and output-buffer behavior moved into
`std::http2::core`, `std::http2::frame`, and `std::http2::hpack`. Production
receive and send paths use the public HPACK codec, and failed immutable
transitions preserve caller-owned state and output.

The broad fixture implementation and its case manifest were removed. Focused
`http2-core-*` executable cases now record public state, branch, byte, and
diagnostic projections. Focused `http2-protocol-core-*` cases remain only when
their human or JSON diagnostics are current observable behavior; they do not
restore the broad fixture.

The historical row inventory and generated retirement checks were temporary
migration gates. After public standard-package tests and executable
specification cases passed without reading those artifacts, the inventory,
scenario model, coverage report, generator, checker, and generated retirement
tests were removed. The final boundary and verification routes are recorded in
[`http2-standard-library-completion-and-fixture-retirement.md`](http2-standard-library-completion-and-fixture-retirement.md).
