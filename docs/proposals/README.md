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
  field-local validation plus decoded-field single-record mapping decode
  helper slices with schema-local field reference, record construction, ADT
  constructor construction mapping expressions, pure same-module
  representation conversion hooks, decoded-field integer equality mapping
  selection, and focused mapping selection diagnostics.
  The implemented source-surface slice also includes top-level public schema
  member aliases for re-exporting existing public schemas through schema-aware
  lookup.
- [Binary Data Standard Library](binary-data-standard-library.md): define the
  remaining binary-buffer, schema-facing conversion, and protocol-facing
  diagnostic behavior beyond the implemented byte vocabulary, byte-view, fixed
  big-endian and little-endian read/write, bounded view buffer helper,
  view-to-chunk materialization, outgoing chunk-list, stream-input, pending
  input and outgoing immutable chunk collection for protocol examples,
  byte-view freeze preservation across task and channel boundaries, and schema
  byte-preview diagnostic slices plus HTTP/2 client connection preface
  protocol byte previews.
- [Binary Schema Primitives And Dispatch](binary-schema-primitives-and-dispatch.md):
  define remaining general binary schema primitive and dispatch behavior.
  Implemented slices include source-surface exact-width and
  `ReservedBits(width, value)` declarations, frame-header and width-sample
  primitive decode, `UInt16le`, `UInt24le`, and `UInt32le` little-endian
  primitive decode and encode, byte-aligned reserved-bit decode and encode,
  one-byte packed reserved-bit decode and encode,
  opt-in `Flag8` one-byte visible flag bitset decode and encode,
  visible fixed exact-width field mismatch diagnostics for generated schema
  decode helpers, exact-width primitive encode, the narrow HTTP/2 payload
  boundary helper, and narrow closed-dispatch and extension-dispatch
  primitive, same-module nested, and imported public nested payload helpers.
  The nested payload helper slices route selected nested payload schemas
  through the same generated binary schema helper path used for ordinary
  generated schema fields.
- [Codec Execution Boundary](codec-execution-boundary.md): define remaining
  executable decode and encode behavior beyond the implemented codec
  declaration source-surface slice, decode function signature boundary,
  mapped decode value boundary, encode function return and mapped value
  parameter boundaries, derived codec mapping value boundary rejections,
  source-visible decode and encode result vocabulary, generated binary schema
  decode-step helper slice for implemented exact-width and same-module nested
  dispatch payload boundaries, hand-written codec decode consumed-count
  validation, and hand-written codec encode and decode execution boundaries
  plus eligible derived codec decode and encode execution boundaries.
- [Schema And Protocol Diagnostics](schema-and-protocol-diagnostics.md):
  define remaining structured diagnostics beyond the implemented closed-input
  `ByteView` read truncation, schema fixed-field mismatch, frame-header schema
  truncation, reserved-bit mismatch, payload length boundary, field-local
  schema validation details, structured schema byte previews, and the HTTP/2
  client connection preface failures, frame-size, header-list-size, and
  flow-control peer-limits, SETTINGS value range peer-limit, stream id domain
  failures, invalid
  connection-state and stream-state frame-kind failures, fixed payload-length
  protocol projections, post-GOAWAY stream rejection projection, and generated
  binary schema encode value-representation failures, generated binary schema
  decode integer range failures, plus hand-written codec decode consumed-count
  failures.
- [HTTP/2 Sans-I/O Protocol Core](http2-sans-io-protocol-core.md): define the
  remaining concrete pure protocol-core behavior beyond the implemented
  ordinary-source decode-state fixture slice, client connection preface
  validation slice, frame-size peer-limit diagnostic slice with receive-limit
  provenance, header-list-size peer-limit diagnostic slice with receive-limit
  provenance, peer-received SETTINGS state for enable push, maximum frame size,
  maximum concurrent streams, initial window size, header table size, and
  maximum header list size, unknown SETTINGS identifier handling, SETTINGS
  value range diagnostic slice, SETTINGS ACK receive slice, invalid
  frame-kind diagnostic slice, HEADERS/CONTINUATION opaque header-block
  preservation slice, HPACK fixture codec boundary slice,
  unknown extension-frame preservation slice, PING/GOAWAY
  receive slice, DATA and `WINDOW_UPDATE` receive flow-control slices,
  peer-created stream admission with concurrent-stream receive-limit
  diagnostics, stream id domain diagnostic slice, GOAWAY last-stream-id
  enforcement for later peer-created HEADERS, `RST_STREAM` receive slice,
  PRIORITY receive slice with dependency facts and self-dependency diagnostic,
  `SETTINGS_INITIAL_WINDOW_SIZE` open-stream receive-window adjustment, and
  inbound DATA and HEADERS `END_STREAM` closed-by-peer stream lifecycle,
  peer-sent `PUSH_PROMISE` rejection on the server receive boundary, outbound
  frame-header encode, outbound SETTINGS ACK send intent, outbound PING ACK
  send intent, narrow outbound DATA flow-control send intent, and outbound
  `WINDOW_UPDATE` receive-credit intent, `RST_STREAM` reset send-intent, and
  GOAWAY send-intent slices.
- [Network Effect Integration Boundary](network-effect-integration-boundary.md):
  define remaining transport adapter, production socket lifecycle,
  stream-routing, richer deadline, cancellation, channel, and task behavior
  beyond the implemented descriptor-backed `net` and `time` boundary calls,
  first fixture-backed listener/stream calls, first transport-error,
  timeout-expiry, and deadline-expiry runtime failure slices, and the
  source-level stream event/action handler boundary examples plus the narrow
  socket-to-handler routing slice.

## Update When

- New proposal work is added, split, superseded, completed, or removed.
- Proposal work becomes implemented and the resulting behavior is documented
  under `../specification/`.
- A completed proposal record moves to
  `../reference/implemented-proposals/`.
