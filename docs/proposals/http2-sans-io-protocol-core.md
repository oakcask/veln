# HTTP/2 Standard Library Completion and Fixture Retirement

Status: proposed

This proposal now tracks only the remaining evidence and stale-route cleanup
needed after the broad `../../examples/specification/run/http2-protocol-core/`
case was retired. Current implemented HTTP/2 behavior belongs under
`../specification/http2.md` and the focused executable cases it routes to.

The standard `http2::core` and `http2::hpack` implementations own the reusable
connection, stream, HPACK, receive, send, flow-control, content-length,
shutdown, and output-buffer transitions. The standard HTTP/2 sources no longer
use `HpackFixtureState`, `hpack_fixture::`, or
`hpack.fixture.unsupported_header_block`.

## Remaining Gate

The fixture deletion gate is still not proven because the checked replacement
matrix was not preserved when the aggregate case was removed. The parent of
the retirement change records:

- 65 distinct `require_*` helper definitions and 717 whole-name occurrences,
  leaving 652 helper invocations after definitions are excluded;
- one exact stdout value with 2,044 newline-terminated output lines;
- 315 `output_chunk_list` assertion tables; and
- 28 focused Veln files under `../../examples/specification/` that still carry
  a fixture-state, fixture-namespace, or fallback-diagnostic marker; these are
  classified in
  [http2-sans-io-fixture-marker-classification.md](http2-sans-io-fixture-marker-classification.md).

The aggregate helper, stdout, and output-table assertions are classified in
[http2-sans-io-aggregate-assertion-classification.md](http2-sans-io-aggregate-assertion-classification.md).
The focused fixture markers are classified in
[http2-sans-io-fixture-marker-classification.md](http2-sans-io-fixture-marker-classification.md).
Both artifacts consolidate rows only where they name the shared invariant and
preserve endpoint role, starting state, diagnostic precedence, result
projection, and emitted bytes.

## Evidence Status

The current receive-loop evidence covers ordered same-chunk SETTINGS and PING,
partial buffering, same-call complete frames followed by a partial suffix, DATA
state updates, PRIORITY application, later invalid PING rejection, caller-owned
output preservation, and later-frame rejection after HPACK, continuation,
flow-control plus content-length, or shutdown state advanced locally. The
aggregate assertion classification maps the retired helper invocations, exact
stdout lines, and output tables to retained focused evidence with zero
unclassified entries.

## Documentation Status

Current-behavior claims that treated the retired aggregate directory or its
deleted files as executable evidence have been replaced with focused routes or
explicit historical context. In particular:

- implemented proposal records that cited
  `../../examples/specification/run/http2-protocol-core/` as active evidence
  are narrowed to historical aggregate evidence;
- direct links to deleted `main.veln` and `case.toml` files are removed;
- focused `http2-protocol-core-*` diagnostic cases may remain current evidence
  when their directories still exist; and
- the retired route README points to this active proposal until the final
  verification and archival pass.

## Completion Steps

1. Confirm `../specification/http2.md` routes only implemented behavior to
   existing focused evidence.
2. Archive this proposal again only after all guarded standard-package,
   protocol-semantics, loader, performance, and workspace verification gates
   pass.
