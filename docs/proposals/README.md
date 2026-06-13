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
  `schema` and `pub schema` declarations, field-local `where`, and binary
  schema primitive declaration slices, structural mapping clauses, codec
  declaration schema import/reference visibility checks, and generated
  exact-width field-local validation and single-record mapping decode helper
  slices.
- [Binary Data Standard Library](binary-data-standard-library.md): define the
  remaining binary-buffer, schema-facing conversion, and protocol-facing
  diagnostic behavior beyond the implemented byte vocabulary, byte-view, fixed
  big-endian read/write, stream-input, and schema byte-preview diagnostic
  slices.
- [Binary Schema Primitives And Dispatch](binary-schema-primitives-and-dispatch.md):
  define remaining binary schema primitive and dispatch behavior beyond the
  implemented source-surface exact-width and `ReservedBits(width, value)`
  declaration slices, the frame-header and width-sample primitive decode
  slices, the exact-width and supported reserved-bit primitive encode helper
  slices, the narrow HTTP/2 payload boundary helper, the narrow closed
  dispatch failure and same-module nested payload slices, and the narrow
  extension-tolerant dispatch preservation and same-module nested payload
  slices.
- [Codec Execution Boundary](codec-execution-boundary.md): define remaining
  executable decode and encode behavior beyond the implemented codec
  declaration source-surface slice, decode function signature boundary,
  mapped decode value boundary, encode function return and mapped value
  parameter boundaries, source-visible decode and encode result vocabulary,
  generated exact-width binary schema decode-step helper slice, and
  hand-written codec encode and decode execution boundaries plus eligible
  derived codec decode and encode execution boundaries.
- [Schema And Protocol Diagnostics](schema-and-protocol-diagnostics.md):
  define remaining structured diagnostics beyond the implemented closed-input
  `ByteView` read truncation, schema fixed-field mismatch, frame-header schema
  truncation, reserved-bit mismatch, payload length boundary, field-local
  schema validation details, structured schema byte previews, and the HTTP/2
  client connection preface failures, frame-size and flow-control peer-limits,
  SETTINGS value range peer-limit, invalid connection-state and stream-state
  frame-kind failures, and fixed payload-length protocol projections.
- [HTTP/2 Sans-I/O Protocol Core](http2-sans-io-protocol-core.md): define the
  remaining concrete pure protocol-core behavior beyond the implemented
  ordinary-source decode-state fixture slice, client connection preface
  validation slice, frame-size peer-limit diagnostic slice with receive-limit
  provenance, SETTINGS value range diagnostic slice, invalid frame-kind
  diagnostic slice, PING/GOAWAY receive slice, and DATA and `WINDOW_UPDATE`
  receive flow-control slices.
- [Network Effect Integration Boundary](network-effect-integration-boundary.md):
  define the later route from pure protocol code to transport effects,
  deadlines, channels, and stream tasks.

## Update When

- New proposal work is added, split, superseded, completed, or removed.
- Proposal work becomes implemented and the resulting behavior is documented
  under `../specification/`.
- A completed proposal record moves to
  `../reference/implemented-proposals/`.
