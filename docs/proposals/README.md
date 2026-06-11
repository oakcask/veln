# Proposals

This directory catalogs planned or accepted work that is not fully documented
as current behavior under `../specification/`. Proposal text is not current
language behavior unless the matching specification page also states it.

Use this page as a catalog only. Pick the proposal that matches the task, then
compare it with `../specification/` before changing behavior.

## Catalog

- [HTTP/2 Binary Schema Design Driver](http2-binary-schema-design-driver.md):
  use an HTTP/2 sans-I/O server core to drive binary schema, codec, and
  standard-library design.
- [Schema Declaration Surface](schema-declaration-surface.md): define
  remaining schema declaration behavior beyond the implemented top-level
  `schema` and `pub schema` declarations and field-local `where`
  source-surface slice.
- [Binary Data Standard Library](binary-data-standard-library.md): define the
  remaining binary-buffer, schema-facing conversion, and protocol-facing
  diagnostic behavior beyond the implemented byte vocabulary, byte-view, fixed
  big-endian read/write, and stream-input slices.
- [Binary Schema Primitives And Dispatch](binary-schema-primitives-and-dispatch.md):
  define exact-width fields, endian-aware fields, reserved bits,
  length-dependent payloads, tag dispatch, and unknown tag preservation.
- [Codec Execution Boundary](codec-execution-boundary.md): separate schema
  declarations from executable decode and encode behavior.
- [Schema And Protocol Diagnostics](schema-and-protocol-diagnostics.md):
  define remaining structured diagnostics beyond the implemented closed-input
  `ByteView` read truncation details.
- [Binary Fixture Helpers](binary-fixture-helpers.md): define remaining binary
  fixture helper work beyond compact hex byte chunks, structured fixture text
  validation details, and the implemented executable-specification named
  fixture records with named truncated-input coverage.
- [HTTP/2 Sans-I/O Protocol Core](http2-sans-io-protocol-core.md): define the
  concrete pure protocol-core slice that exercises the binary schema work.
- [Network Effect Integration Boundary](network-effect-integration-boundary.md):
  define the later route from pure protocol code to transport effects,
  deadlines, channels, and stream tasks.

## Update When

- New proposal work is added, split, superseded, completed, or removed.
- Proposal work becomes implemented and the resulting behavior is documented
  under `../specification/`.
- A completed proposal record moves to
  `../reference/implemented-proposals/`.
