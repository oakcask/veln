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
  constructor construction mapping expressions, pure same-module and imported
  public representation conversion hooks whose arguments may be schema-local
  fields or structural mapping expressions, field selection from record-shaped
  structural mapping expressions, decoded-field integer mapping arithmetic,
  decoded-field integer equality mapping selection, focused mapping selection
  diagnostics, and the generated-helper schema validation diagnostic boundary, generated
  `validate_<schema>` decoded-record validation boundary, plus direct
  structural mapped schema encode helper, generated encode-time field-local
  validation for eligible schema helpers, derived encode boundary support, and
  codec decode boundaries
  over multiple decoded-field selected mappings that resolve to one mapped
  record shape.
  The implemented source-surface slice also includes top-level public schema
  member aliases for re-exporting existing public schemas through schema-aware
  lookup and documentation comments that reference schemas through
  schema-aware lookup.
- [Binary Data Standard Library](binary-data-standard-library.md): define the
  remaining binary-buffer, schema-facing conversion, and protocol-facing
  diagnostic behavior beyond the implemented byte vocabulary, byte-view, fixed
  big-endian and little-endian read/write, bounded view buffer helper,
  view-to-chunk materialization, outgoing chunk-list, stream-input, pending
  input and outgoing immutable chunk collection for protocol examples,
  byte-view freeze preservation across task and channel boundaries, and schema
  byte-preview diagnostic slices plus HTTP/2 protocol byte previews.
- [Binary Schema Primitives And Dispatch](binary-schema-primitives-and-dispatch.md):
  define remaining general binary schema primitive and dispatch behavior.
  Implemented slices include source-surface exact-width and
  `ReservedBits(width, value)` declarations, generated `Http2FrameHeaderWire`
  helper decode used by the HTTP/2 protocol-core frame-header path,
  width-sample primitive decode, `UInt16le`, `UInt24le`,
  `UInt31le`, `UInt32le`, and `UInt64le` little-endian primitive decode and
  encode, `UInt64be` big-endian primitive decode and encode,
  byte-aligned reserved-bit decode and encode,
  one-byte, two-byte, three-byte, and four-byte packed reserved-prefix decode
  and encode,
  one-byte, two-byte, three-byte, and four-byte packed reserved-suffix decode
  and encode,
  non-byte-aligned middle `UIntN` plus `ReservedBits(width, value)` plus
  `UIntN` decode and encode,
  one-byte non-byte-aligned reserved prefix groups followed by two visible
  `UIntN` fields, and consecutive non-byte-aligned `UIntN` and
  `ReservedBits(width, value)` groups that complete one byte or one
  two-byte, three-byte, or four-byte big-endian storage unit,
  opt-in `Flag8` one-byte, `Flag16be` two-byte big-endian, `Flag16le`
  two-byte little-endian, `Flag32be` four-byte big-endian, and `Flag32le`
  four-byte little-endian visible flag
  bitset decode and encode, checked bit and raw-bit helpers,
  structural mapping decode, direct mapped-record encode, and direct ADT
  constructor mapped encode boundaries for supported schema-local fields plus
  one record-payload constructor slice,
  standalone visible `UInt1` through `UInt7` decode and encode,
  bounded `Repeat(count_field, Payload)` and
  `Repeat(left_count - right_count, Payload)` primitive and nested schema field
  decode and encode slices, bounded `Repeat(count_field, ByteView(length_field))`
  decode and encode plus derived codec boundary slices, length-bounded `ByteView(length_field)` and
  `ByteView(left_length - right_length)` decode and encode,
  schema-level structural validation for decoded `Int` fields,
  visible fixed exact-width field mismatch diagnostics for generated schema
  decode helpers, exact-width primitive encode, the narrow HTTP/2 payload
  boundary helper, and narrow closed-dispatch and extension-dispatch
  primitive, same-module nested, and imported public nested payload helpers.
  The nested payload helper slices route selected nested payload schemas
  through the same generated binary schema helper path used for ordinary
  generated schema fields, and checked non-HTTP coverage combines the
  implemented helper vocabulary in one decode-and-encode schema.
- [Codec Execution Boundary](codec-execution-boundary.md): define remaining
  executable decode and encode behavior beyond the implemented codec
  declaration source-surface slice, decode function signature boundary,
  mapped decode value boundary, encode function return and mapped value
  parameter boundaries, derived codec mapping value boundary checks,
  source-visible decode and encode result vocabulary, generated binary schema
  decode-step helper slice for implemented exact-width, repeat-backed, and
  same-module and public imported nested dispatch payload boundaries,
  hand-written codec decode consumed-count validation, hand-written codec
  encode and decode execution boundaries including source-visible partial
  encode preservation and resume, plus eligible derived codec decode and
  encode execution boundaries over the checked non-HTTP composite helper
  shape.
- [Schema And Protocol Diagnostics](schema-and-protocol-diagnostics.md):
  define remaining structured diagnostics beyond the implemented closed-input
  `ByteView` read truncation, schema fixed-field mismatch, frame-header schema
  truncation, reserved-bit mismatch, payload length boundary, field-local
  schema validation details, structured schema byte previews, and the HTTP/2
  client connection preface failures, frame-size, header-list-size, and
  flow-control peer-limits, SETTINGS value range peer-limit, stream id domain
  failures, invalid
  connection-state and stream-state frame-kind failures, fixed payload-length
  protocol projections with protocol-owned payload byte previews, the explicit
  HTTP/2 invalid DATA padding projection, the explicit
  HTTP/2 protocol diagnostic projection boundary for representative protocol
  and peer-limit failures, post-GOAWAY stream rejection projection, and
  generated
  binary schema encode value-representation failures, generated `EncodeError`
  command-facing projection for encode value, dispatch unknown tag, dispatch
  length mismatch, and dispatch mismatch failures, generated binary schema
  decode integer range failures, generated bounded repeated schema field
  truncation diagnostics with indexed field paths in JSON and human output,
  plus hand-written codec decode consumed-count failures.
- [HTTP/2 Sans-I/O Protocol Core](http2-sans-io-protocol-core.md): define the
  remaining concrete pure protocol-core behavior beyond the implemented
  ordinary-source decode-state fixture slice, client connection preface
  validation slice, frame-size peer-limit diagnostic slice with receive-limit
  provenance, header-list-size peer-limit diagnostic slice with receive-limit
  provenance, peer-received SETTINGS state for enable push, maximum frame size,
  maximum concurrent streams, initial window size, header table size, and
  maximum header list size, unknown SETTINGS identifier handling, SETTINGS
  value range diagnostic slice, SETTINGS ACK receive and outstanding-local
  SETTINGS tracking slice, invalid
  frame-kind diagnostic slice, HEADERS/CONTINUATION opaque header-block
  preservation slice, HPACK fixture codec boundary slice including the static
  indexed `0x82` `:method: GET`, `0x83` `:method: POST`, `0x84` `:path: /`,
  `0x85` `:path: /index.html`, `0x86` `:scheme: http`, and `0x87`
  `:scheme: https`, plus `0x88` `:status: 200`, `0x89` `:status: 204`,
  `0x8a` `:status: 206`, `0x8b` `:status: 304`, `0x8c` `:status: 400`,
  `0x8d` `:status: 404`, `0x8e` `:status: 500`, `0x8f`
  `accept-charset:`, `0x90` `accept-encoding: gzip, deflate`, and `0x91`
  `accept-language:` bytes,
  unknown extension-frame preservation slice, PING/GOAWAY
  receive slice, DATA and `WINDOW_UPDATE` receive flow-control slices,
  PADDED DATA receive handling with invalid-padding diagnostics,
  peer-created stream admission with concurrent-stream receive-limit
  diagnostics, stream id domain diagnostic slice including HEADERS and
  CONTINUATION on the connection stream, GOAWAY last-stream-id enforcement
  for later peer-created HEADERS and local outbound HEADERS send-intents,
  `RST_STREAM` receive slice,
  PRIORITY receive slice with dependency facts, tracked open-stream priority
  state replacement, stream-state failure preservation, and self-dependency
  diagnostic,
  `SETTINGS_INITIAL_WINDOW_SIZE` open-stream receive-window adjustment, and
  inbound DATA and HEADERS `END_STREAM` closed-by-peer stream lifecycle,
  peer-sent `PUSH_PROMISE` rejection on the server receive boundary, outbound
  frame-header encode, outbound SETTINGS ACK send intent, local SETTINGS
  send intents for `SETTINGS_HEADER_TABLE_SIZE`,
  `SETTINGS_INITIAL_WINDOW_SIZE`, `SETTINGS_ENABLE_PUSH`,
  `SETTINGS_MAX_CONCURRENT_STREAMS`, `SETTINGS_MAX_FRAME_SIZE`, and
  `SETTINGS_MAX_HEADER_LIST_SIZE`, outbound PING ACK send intent, narrow
  outbound DATA frame-header-plus-payload send intent with `END_STREAM` local
  closed-stream state, and outbound `WINDOW_UPDATE` receive-credit intent,
  `RST_STREAM` reset send-intent, outbound PRIORITY send-intent, outbound
  HEADERS send-intent, and GOAWAY send-intent slices.
- [Network Effect Integration Boundary](network-effect-integration-boundary.md):
  define remaining transport adapter, production socket lifecycle, richer
  stream-routing, richer deadline, cancellation, channel, and task behavior
  beyond the implemented transport, route-count, task, deadline,
  cancellation, deadline-aware listener accept, deadline-aware stream read,
  adapter-owned listener-to-clean-stream-end lifecycle, network task
  two-argument spawn, and deadline-aware accepted-stream lifecycle slices
  documented under `../specification/`; completed proposal records live under
  `../reference/implemented-proposals/`.

## Update When

- New proposal work is added, split, superseded, completed, or removed.
- Proposal work becomes implemented and the resulting behavior is documented
  under `../specification/`.
- A completed proposal record moves to
  `../reference/implemented-proposals/`.
