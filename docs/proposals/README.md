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
  constructor construction mapping expressions including nested constructor
  payloads in generated decode mappings, pure same-module and imported
  public representation conversion hooks that take one, two, three, four, or five
  arguments from schema-local fields or structural mapping expressions, field
  selection from record-shaped structural mapping expressions, decoded-field,
  integer-literal, and `Int` converter-call mapping arithmetic including
  integer division, equality, inequality, and ordered mapping comparisons over
  supported `Int` mapping operands for `Bool` target fields composed with
  `and`, `or`, and `not`, narrow boolean mapping selection, focused mapping
  selection diagnostics, and the generated-helper schema validation diagnostic boundary,
  generated `validate_<schema>` decoded-record validation boundary, plus
  projectable structural mapped schema encode helper including explicitly
  named same-module and imported converter inverse projection, generated encode-time
  field-local validation for eligible schema helpers, derived encode boundary
  support, derived selected-mapping encode boundary support, and codec decode
  boundaries over multiple decoded-field selected mappings that resolve to one
  mapped record shape.
  The implemented source-surface slice also includes top-level public schema
  member aliases for re-exporting existing public schemas through schema-aware
  lookup and documentation comments that reference schemas through
  schema-aware lookup. Binary fixture metadata in executable specification
  cases may also validate schema-aware references.
- [Binary Data Standard Library](binary-data-standard-library.md): define the
  remaining binary-buffer, schema-facing conversion, and protocol-facing
  diagnostic behavior beyond the implemented byte vocabulary, byte-view, fixed
  big-endian and little-endian read/write through the current source-visible
  helper width set, bounded view buffer helper,
  view-to-chunk materialization, outgoing chunk-list, stream-input, pending
  input and outgoing immutable chunk collection for protocol examples,
  byte-view freeze preservation across task and channel boundaries,
  source-visible `ByteView` range diagnostics with byte previews, checked byte
  write conversion diagnostics, and schema byte-preview diagnostic slices plus
  HTTP/2 client preface, invalid frame-kind, and PRIORITY self-dependency
  protocol byte previews, plus the HPACK fixture unsupported-header-block
  and malformed-Huffman-padding protocol byte previews, HTTP/2 SETTINGS value
  range protocol byte preview, HTTP/2 DATA receive flow-control protocol byte
  preview, HTTP/2 `WINDOW_UPDATE` invalid-increment protocol byte preview, and
  HTTP/2 unexpected SETTINGS ACK protocol byte preview, plus HTTP/2
  header-list and header-table receive-limit protocol byte previews.
- [Binary Schema Primitives And Dispatch](binary-schema-primitives-and-dispatch.md):
  define remaining general binary schema primitive and dispatch behavior.
  Implemented slices include source-surface exact-width and
  `ReservedBits(width, value)` declarations, generated `Http2FrameHeaderWire`
  helper decode used by the HTTP/2 protocol-core frame-header path,
  width-sample primitive decode, `UInt16le`, `UInt24le`,
  `UInt31le`, `UInt32le`, `UInt40le`, `UInt48le`, `UInt56le`, and `UInt64le`
  little-endian primitive decode and encode, `UInt40be` five-byte,
  `UInt48be` six-byte, `UInt56be` seven-byte, and `UInt64be` eight-byte
  big-endian primitive decode and encode,
  byte-aligned reserved-bit decode and encode,
  one-byte, two-byte, three-byte, and four-byte packed reserved-prefix decode
  and encode,
  one-byte, two-byte, three-byte, and four-byte packed reserved-suffix decode
  and encode,
  non-byte-aligned middle `UIntN` plus `ReservedBits(width, value)` plus
  `UIntN` decode and encode, including the narrow two-byte byte-interleaved
  middle layout,
  one-byte, two-byte, three-byte, four-byte, five-byte, and six-byte
  reserved prefix groups followed by two visible `UIntN` fields, including
  two-byte reserved prefix widths one through fourteen, three-byte reserved
  prefix widths seventeen through twenty-three, four-byte reserved prefix
  widths twenty-five through thirty-one, five-byte reserved prefix width
  thirty-three, and six-byte reserved prefix width forty-one, and
  consecutive non-byte-aligned
  `UIntN` and
  `ReservedBits(width, value)` groups that complete one byte or one
  two-byte, three-byte, four-byte, five-byte, six-byte, seven-byte, or
  eight-byte big-endian storage unit,
  opt-in `Flag8` one-byte, `Flag16be` two-byte big-endian, `Flag16le`
  two-byte little-endian, `Flag24be` three-byte big-endian, `Flag24le`
  three-byte little-endian, `Flag32be` four-byte big-endian, `Flag32le`
  four-byte little-endian, `Flag40be` five-byte big-endian, `Flag40le`
  five-byte little-endian, `Flag48be` six-byte big-endian, `Flag48le`
  six-byte little-endian, `Flag56be` seven-byte big-endian, `Flag56le`
  seven-byte little-endian, `Flag64be` eight-byte big-endian, and `Flag64le`
  eight-byte little-endian visible flag
  bitset decode and encode, checked bit and raw-bit helpers,
  structural mapping decode including constructor payload field selection
  from record-shaped mapping expressions, projectable mapped-record encode,
  same-module and imported converter-call mapped encode with explicitly named
  inverse converters, and direct or nested ADT constructor mapped encode
  boundaries for supported schema-local fields plus record-payload
  constructor slices,
  standalone visible `UInt1` through `UInt7` decode and encode,
  visible-only packed `UInt1` through `UInt7` one-byte group decode and
  encode,
  bounded `Repeat(count_field, Payload)` and
  `Repeat(left_count - right_count, Payload)` primitive, same-module nested
  schema field, and public imported nested schema field decode and encode
  slices, bounded `Repeat(left_count + right_count,
  Payload)` decode and encode with primitive count-mismatch and derived codec
  boundary coverage, bounded `Repeat(left_count * right_count, Payload)`
  decode and encode with count-mismatch and invalid-count coverage, bounded
  `Repeat(left_count / right_count, Payload)` decode and encode plus
  division-by-zero coverage,
  bounded `Repeat(count_field, ByteView(length_field))` decode and encode plus
  derived codec boundary slices, length-bounded
  `ByteView(length_field)`, `ByteView(left_length - right_length)`, and
  `ByteView(left_length + right_length)`, and
  `ByteView(left_length * right_length)`, and
  `ByteView(left_length / right_length)` decode and encode,
  declaration-time missing, forward, and wrong-role schema-local field
  reference diagnostics for repeat count fields and count expressions,
  byte-view lengths, dispatch tags, and extension-dispatch tags and lengths,
  schema-level structural validation for decoded `Int` fields,
  visible fixed exact-width field mismatch diagnostics for generated schema
  decode helpers, exact-width primitive encode, the HTTP/2 GOAWAY payload
  schema encode boundary, and narrow closed-dispatch and extension-dispatch
  primitive payload helpers plus eligible nested payload helpers.
  The nested payload helper slices accept eligible nested payload schemas that
  use the same generated binary schema helper path used for ordinary generated
  schema fields, including supported representation-only reserved-bit layouts,
  and checked non-HTTP coverage combines the implemented helper vocabulary in
  one decode-and-encode schema. Closed dispatch payload cases
  with mixed primitive and nested decoded shapes are implemented for selected
  mappings keyed by the dispatch tag field when those mappings cover the
  dispatch cases and resolve to one target record shape. Same-module and
  public imported recursive closed-dispatch and extension-dispatch payload
  decode and encode support is implemented for the length-bounded forms when
  selected mappings cover every known case, resolve to one target record
  shape, and include a non-recursive base case. Focused dispatch payload
  diagnostics now also name the generated decode and encode helper boundaries
  for resolved binary nested payload schemas that cannot expose those helpers,
  including `ByteView` payload layouts whose length field is not an earlier
  decoded `Int` field. The completed nested schema
  repeat payload helper slice is archived under
  [Binary Schema Repeat Schema Payload Helpers](../reference/implemented-proposals/binary-schema-repeat-schema-payload-helpers.md).
  The completed `UInt56be` and
  `UInt56le` exact-width primitive slice is archived under
  [Binary Schema UInt56 Primitives](../reference/implemented-proposals/binary-schema-u56-primitives.md).
  The completed `Flag40be`, `Flag40le`, `Flag56be`, and `Flag56le` flag
  bitset slice is archived under
  [Binary Schema Flag40 And Flag56 Bitsets](../reference/implemented-proposals/binary-schema-flag40-and-flag56-bitsets.md).
  The completed `Flag48be` and `Flag48le` flag bitset slice is archived under
  [Binary Schema Flag48 Bitsets](../reference/implemented-proposals/binary-schema-flag48-bitsets.md).
  The completed reserved-byte-prefix encode slice for `ReservedBits(2, 0)`
  and `ReservedBits(9, 0)` followed by `UInt8` is archived under
  [Binary Schema Reserved Byte Prefix Encode](../reference/implemented-proposals/binary-schema-reserved-byte-prefix-encode.md).
- [Codec Execution Boundary](codec-execution-boundary.md): define remaining
  executable decode and encode behavior beyond the implemented codec
  declaration source-surface slice, decode function signature boundary,
  mapped decode value boundary, encode function return and mapped value
  parameter boundaries, derived codec mapping value boundary checks,
  source-visible decode and encode result vocabulary, generated binary schema
  decode-step helper slice for implemented exact-width, middle reserved,
  repeat-backed, and same-module and public imported nested dispatch payload
  boundaries,
  hand-written codec decode consumed-count validation, hand-written codec
  encode and decode execution boundaries including caller-owned parser-state
  retention around `Decoded` and `NeedMore`, the bounded `ByteView` plus base
  `ByteOffset` hand-written decode example with non-consuming short-input
  readiness and absolute malformed-input offsets, source-visible partial
  encode preservation and resume, plus eligible derived codec decode and
  encode execution boundaries, including budgeted derived encode, over the
  checked non-HTTP composite helper shape and general generated helper shape,
  same-module recursive closed and extension dispatch payload helpers, and
  selected structural mapping encode slice, and derived helper eligibility
  diagnostics for unsupported generated decode and encode directions.
- [Schema And Protocol Diagnostics](schema-and-protocol-diagnostics.md):
  define remaining structured diagnostics beyond the implemented closed-input
  `ByteView` read truncation, schema fixed-field mismatch, frame-header schema
  truncation, reserved-bit mismatch, payload length boundary, field-local
  schema validation details, structured schema byte previews, and the HTTP/2
  client connection preface failures, frame-size and header-list-size
  peer-limits, flow-control peer-limits with protocol-owned DATA payload byte
  previews, SETTINGS value range peer-limit, stream id domain failures with
  protocol-owned frame-header byte previews, invalid
  connection-state and stream-state frame-kind failures with protocol-owned
  frame-header byte previews, fixed payload-length protocol projections with
  protocol-owned payload byte previews, the explicit
  HTTP/2 invalid DATA padding projection, the HTTP/2 PRIORITY self-dependency
  projection with protocol-owned payload byte preview, the HTTP/2 unexpected
  SETTINGS ACK projection with protocol-owned frame-header byte preview, the
  HTTP/2 continuation-ordering projection with protocol-owned frame-header
  byte preview, the HTTP/2 pending-byte close projection with retained-byte
  preview, the
  explicit
  HTTP/2 protocol diagnostic projection boundary for focused protocol and
  peer-limit failures, including post-GOAWAY stream rejection, fixed
  payload-length, invalid DATA padding, SETTINGS ACK state, preface,
  continuation, and invalid frame-kind fixtures, and
  generated
  binary schema encode value-representation failures, generated `EncodeError`
  command-facing projection for encode value, dispatch unknown tag, dispatch
  length mismatch, and dispatch mismatch failures, command-facing projection
  for `EncodeStep::Invalid(EncodeError(...))` entry results,
  command-facing projection for
  `DecodeStep::Invalid(DecodeError(...))` and `DecodeStep::NeedMore(...)`
  entry results, reason-carrying hand-written codec invalid-input decode
  projection with optional carried byte-helper context, generated binary
  schema decode integer range failures, generated bounded repeated schema
  field truncation diagnostics with indexed field paths in JSON and human
  output, plus hand-written codec decode consumed-count failures and their
  command-facing projection.
- [HTTP/2 Sans-I/O Protocol Core](http2-sans-io-protocol-core.md): define the
  remaining concrete pure protocol-core behavior beyond the implemented
  ordinary-source receive-state, diagnostics, settings, stream lifecycle,
  HPACK behavior beyond the checked fixture boundary,
  unknown extension-frame, receive flow-control, send-intent,
  `RST_STREAM`, PRIORITY, PING, GOAWAY, local GOAWAY outbound HEADERS
  boundary, server-side `PUSH_PROMISE` rejection, server-side outbound
  `PUSH_PROMISE` send-intent, and
  half-closed-by-peer outbound DATA send-intent slices,
  request-side, response-side, and request-trailer header-list validation,
  the source-visible `te` header value rule, the `content-length`
  header-list consistency slice, accepted `content-length` body-length
  accounting for tracked inbound DATA,
  the malformed Huffman padding fixture diagnostic, and the outbound HPACK
  fixture header-list encoder slice, including static-name literal fixtures,
  full-table single-byte Huffman-marked string literal decoding and encoding,
  and the bounded
  stateful dynamic-table fixture encoder path, focused unsupported-Huffman EOS
  and non-visible decoded-byte diagnostics, focused malformed string-length
  and raw string value fixture diagnostics, general visible-ASCII raw literal
  values, raw field-name validation through header-list diagnostics, inbound
  fixture dynamic-table insertion, ordinary raw new-name dynamic-indexed reuse
  and eviction, checked inbound table-size update placement diagnostics, and
  checked outbound dynamic table-size update encoding, plus deterministic
  `hpack-bytes-*` multi-byte non-visible Huffman fixture labels,
  recorded under `../specification/` and
  `../reference/implemented-proposals/`. Planned work still includes
  broader protocol-core behavior and full HPACK behavior beyond the checked
  fixture boundary, including full HPACK compression and unbounded
  dynamic-table behavior.
  The completed half-closed-by-peer outbound DATA send-intent slice is archived
  under
  [HTTP/2 Half-Closed-By-Peer Outbound DATA](../reference/implemented-proposals/http2-half-closed-by-peer-outbound-data.md).
  The completed outbound DATA send-credit refill from peer `WINDOW_UPDATE`
  slice is covered by
  [HTTP/2 Outbound DATA Flow Control](../reference/implemented-proposals/http2-outbound-data-flow-control.md).
  The completed outbound DATA post-GOAWAY send-intent boundary is archived
  under
  [HTTP/2 Outbound DATA GOAWAY Boundary](../reference/implemented-proposals/http2-outbound-data-goaway-boundary.md).
  The completed accepted `content-length` body accounting slice is archived
  under
  [HTTP/2 Content-Length Body Accounting](../reference/implemented-proposals/http2-content-length-body-accounting.md).
- [Network Effect Integration Boundary](network-effect-integration-boundary.md):
  define remaining transport adapter, richer production socket APIs, richer
  stream-routing, richer deadline, cancellation, channel, and task behavior
  beyond the implemented transport, bounded channel-first routing, general
  receiver-list routing, task, deadline, cancellation, deadline-aware listener
  accept, cancellable deadline-aware listener accept, deadline-aware stream
  read, cancellable deadline-aware stream read, cancellable deadline-aware
  stream write, adapter-owned
  listener-to-clean-stream-end lifecycle, context-based
  `task::spawn_with<Result, Context>` handler spawn,
  accepted-stream lifecycle variants for deadline-aware, cancellable, and
  cancellable deadline-aware adapters, stream close lifecycle,
  adapter-owned clean shutdown after cancellation and deadline expiry,
  source-visible ordered `net::write_chunks` chunk-list writes,
  source-visible deadline-aware `net::write_chunk_until` writes,
  source-visible deadline-aware `net::write_chunks_until` chunk-list writes,
  source-visible cancellable deadline-aware
  `net::write_chunks_until_cancellable` chunk-list writes,
  adapter-owned multi-handler outbound write ordering through
  `net::write_chunks`,
  production-loopback listen, sequential accept, read, write, clean listener
  end, close lifecycle, two-stream adapter handler/action lifecycle, and
  listener-drain adapter lifecycle, listener-drain read-failure runtime
  boundary, deadline-aware adapter lifecycle, deadline-aware accept and read
  failure runtime boundaries, production cancellable deadline-aware adapter
  lifecycle and outcome boundary, adapter close-failure runtime boundary, and
  explicit listener-close boundary, adapter-owned cancellation owner
  lifecycle boundary,
  bounded
  receiver-list channel-first routing through the
  current checked select-many boundary, receiver-list cancellable
  channel-first routing,
  timeout-result selection, receiver-list cancellable timeout-result
  selection, two-receiver timeout-result selection, and two-receiver
  cancellable timeout-result selection slices
  documented under `../specification/`; completed proposal records live under
  `../reference/implemented-proposals/`.
  The completed source-visible ordered `net::write_chunks` chunk-list write
  slice is archived under
  [Network Write Chunks Boundary](../reference/implemented-proposals/network-write-chunks-boundary.md).
  The completed deadline-aware stream-write boundary is archived under
  [Network Write Until Boundary](../reference/implemented-proposals/network-write-until-boundary.md).
  The completed deadline-aware chunk-list stream-write boundary is archived
  under
  [Network Write Chunks Until Boundary](../reference/implemented-proposals/network-write-chunks-until-boundary.md).
  The completed cancellable deadline-aware stream-write boundary is archived
  under
  [Network Write Until Cancellable Boundary](../reference/implemented-proposals/network-write-until-cancellable-boundary.md).
  The completed cancellable deadline-aware chunk-list stream-write boundary is
  archived under
  [Network Write Chunks Until Cancellable Boundary](../reference/implemented-proposals/network-write-chunks-until-cancellable-boundary.md).
  The completed explicit listener-close boundary is archived under
  [Network Listener Close Boundary](../reference/implemented-proposals/network-listener-close-boundary.md).
  The completed adapter-owned multi-handler outbound write-ordering slice is
  archived under
  [Network Adapter Outbound Write Ordering](../reference/implemented-proposals/network-adapter-outbound-write-ordering.md).
  The completed adapter-owned clean shutdown slice is archived under
  [Network Adapter Clean Shutdown](../reference/implemented-proposals/network-adapter-clean-shutdown.md).

## Update When

- New proposal work is added, split, superseded, completed, or removed.
- Proposal work becomes implemented and the resulting behavior is documented
  under `../specification/`.
- A completed proposal record moves to
  `../reference/implemented-proposals/`.
