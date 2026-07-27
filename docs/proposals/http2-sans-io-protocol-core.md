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

Completion requires a checked artifact that maps every removed helper
invocation, exact stdout line, and output table to equivalent retained evidence
or to an intentional historical diagnostic boundary. The focused fixture
markers are already classified by the linked artifact. The remaining mapping
may consolidate rows only when it names the shared invariant and preserves
endpoint role, starting state, diagnostic precedence, result projection, and
emitted bytes.

## Missing Evidence

The current receive-loop evidence covers ordered same-chunk SETTINGS and PING,
partial buffering, same-call complete frames followed by a partial suffix, DATA
state updates, PRIORITY application, later invalid PING rejection, and
caller-owned output preservation. The remaining matrix work must still recover
or replace representative rows for:

- later-frame rejection after HPACK state advanced locally;
- later-frame rejection after continuation state advanced locally;
- later-frame rejection after flow-control or content-length state advanced
  locally;
- later-frame rejection after shutdown state advanced locally; and
- any deleted exact stdout or output-chunk assertion that is not already
  protected by focused cases.

Add adjacent standard tests and focused executable cases for any behavior that
the classification cannot map to existing evidence.

## Documentation Cleanup

Replace current-behavior claims that treat the retired aggregate directory or
its deleted files as executable evidence with focused routes or explicit
historical context. In particular:

- implemented proposal records that cite
  `../../examples/specification/run/http2-protocol-core/` as active evidence
  must be narrowed or redirected;
- direct links to deleted `main.veln` and `case.toml` files must be removed;
- focused `http2-protocol-core-*` diagnostic cases may remain current evidence
  when their directories still exist; and
- the retired route README must not point to this proposal as completed until
  the matrix reaches zero unclassified entries.

## Completion Steps

1. Build the checked aggregate assertion classification artifact from the
   parent of the retirement change and keep it in the repository.
2. Add missing standard tests and focused cases identified by that artifact.
3. Reconcile stale implemented-record routes and the retired README route.
4. Confirm `../specification/http2.md` routes only implemented behavior to
   existing focused evidence.
5. Archive this proposal again only after the artifact has zero unclassified
   entries and all guarded standard-package, protocol-semantics, loader,
   performance, and workspace verification gates pass.
