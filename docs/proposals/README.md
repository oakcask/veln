# Proposals

This directory catalogs planned or accepted work that is not fully documented
as current behavior under `../specification/`. Proposal text is not current
language behavior unless the matching specification page also states it.

Use this page as a catalog only. Pick the proposal that matches the task, then
compare it with `../specification/` before changing behavior.

## Catalog

- [Local Inference And Annotation Elision](local-inference-and-annotation-elision.md):
  define remaining annotation-elision work for callback inputs outside the
  compiler-known, concrete declared-helper signature, concrete record-field
  expected-type, concrete local-binding expected-type, and concrete
  direct return-position expected-type, concrete match-arm expected-type, and
  concrete if-branch expected-type, and concrete constructor-payload
  expected-type paths, and other paths beyond the implemented same-function
  local `let`, non-empty collection initializer, empty collection expected-type,
  nested initializer expected-type
  propagation, hole expected-type flow, empty collection callback return,
  payload-carrying ADT constructor inference, match scrutinee
  constructor-pattern inference, local pattern `let` inference,
  compiler-known prelude callback argument including `vec_try_map_with`,
  dictionary callback alias, declared helper callback argument, source-backed
  prelude callback fallback, record-field callback, local callback binding,
  local callback binding annotation elision, direct return callback, match-arm
  callback, if-branch callback, callback return expected-type,
  constructor-payload callback, variadic declared-helper callback parameter,
  and examples cleanup slices specified in
  `../specification/types.md`.
  The completed private helper call-site inference, prelude callback argument
  inference including `vec_try_map_with`, dictionary callback alias inference,
  declared helper callback argument inference, source-backed prelude callback
  fallback, record-field callback inference, local callback binding inference,
  local callback binding annotation-elision inference, direct return callback
  inference, callback return expected-type inference, constructor-payload
  callback inference, non-empty collection initializer inference, ADT constructor
  payload inference, match scrutinee constructor-pattern inference, local
  pattern `let` inference, nested initializer expected-type propagation, hole
  expected-type flow, and examples cleanup slices are archived under
  `../reference/implemented-proposals/local-inference-private-helper-call-site.md`,
  `../reference/implemented-proposals/local-inference-prelude-callback-argument.md`,
  `../reference/implemented-proposals/local-inference-dictionary-callback-aliases.md`,
  `../reference/implemented-proposals/local-inference-declared-helper-callback-argument.md`,
  `../reference/implemented-proposals/local-inference-prelude-callback-fallback.md`,
  `../reference/implemented-proposals/local-inference-record-field-callback.md`,
  `../reference/implemented-proposals/local-inference-local-callback-binding.md`,
  [local-inference-local-callback-binding-annotation-elision.md](../reference/implemented-proposals/local-inference-local-callback-binding-annotation-elision.md),
  `../reference/implemented-proposals/local-inference-direct-return-callback.md`,
  `../reference/implemented-proposals/local-inference-match-arm-callback.md`,
  `../reference/implemented-proposals/local-inference-if-branch-callback.md`,
  `../reference/implemented-proposals/local-inference-callback-return-expected-type.md`,
  `../reference/implemented-proposals/local-inference-constructor-payload-callback.md`,
  [local-inference-variadic-callback-parameter.md](../reference/implemented-proposals/local-inference-variadic-callback-parameter.md),
  [local-inference-non-empty-collection-initializer.md](../reference/implemented-proposals/local-inference-non-empty-collection-initializer.md),
  `../reference/implemented-proposals/local-inference-adt-constructor-payload.md`,
  `../reference/implemented-proposals/local-inference-match-scrutinee-constructor-pattern.md`,
  `../reference/implemented-proposals/local-inference-local-pattern-let.md`,
  [local-inference-nested-initializer-expected-type.md](../reference/implemented-proposals/local-inference-nested-initializer-expected-type.md),
  `../reference/implemented-proposals/local-inference-hole-expected-type-flow.md`,
  and
  `../reference/implemented-proposals/local-inference-examples-cleanup.md`.
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
  public representation conversion hooks that take one or more supported
  arguments from schema-local fields or structural mapping expressions
  including pure converter calls, field
  selection from record-shaped structural mapping expressions, decoded-field,
  integer-literal, and `Int` converter-call mapping arithmetic including
  integer division, equality, inequality, and ordered mapping comparisons over
  supported `Int` mapping operands for `Bool` target fields composed with
  `and`, `or`, and `not`, narrow boolean mapping selection, pure `Bool`
  converter selector calls through same-module functions, written imported
  paths, unqualified public imports, or public function aliases, format-neutral
  schema bodies without a `format` clause plus
  `format binary` gating for binary-only field vocabulary, focused mapping
  selection diagnostics, and the generated-helper schema validation diagnostic boundary,
  generated `validate_<schema>` decoded-record validation boundary, visible
  flag bitset decode bindings, bounded repeat generated helper bindings, plus
  projectable structural mapped schema encode helper including explicitly
  named same-module and imported converter inverse projection through written
  imports, unqualified public imports, and public function aliases, generated encode-time
  field-local validation for eligible schema helpers, derived encode boundary
  support, derived selected-mapping encode boundary support including ordered
  field-literal selector comparisons, and codec decode boundaries over
  multiple decoded-field selected mappings that resolve to one mapped record
  shape. The completed bounded repeat helper binding slice is
  archived under
  [Binary Schema Repeat Helper Bindings](../reference/implemented-proposals/binary-schema-repeat-schema-payload-helpers.md).
  The completed narrow arithmetic mapped encode slice is
  archived under
  [Binary Schema Mapping Arithmetic Encode](../reference/implemented-proposals/binary-schema-mapping-arithmetic-encode.md).
  The implemented source-surface slice also includes top-level public schema
  member aliases for re-exporting existing public schemas through schema-aware
  lookup. The completed documentation-comment schema reference slice is
  archived under
  [Schema Documentation References](../reference/implemented-proposals/schema-documentation-references.md).
  Binary fixture metadata in executable specification cases may also validate
  schema-aware references.
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
  protocol byte previews, HTTP/2 invalid stream-id domain protocol byte
  previews, plus the HPACK fixture unsupported-header-block and
  malformed-Huffman-padding protocol byte previews, HTTP/2 SETTINGS value
  range protocol byte preview, HTTP/2 `WINDOW_UPDATE` invalid-increment
  protocol byte preview, HTTP/2 DATA receive flow-control protocol byte
  preview, HTTP/2 unexpected SETTINGS ACK protocol byte preview, HTTP/2
  invalid DATA padding protocol byte preview, plus HTTP/2 frame-size,
  header-list, header-table, concurrent-stream receive-limit, and
  stream-after-GOAWAY protocol byte previews, and HTTP/2 request, response,
  request-trailer, and response-trailer header-list validation protocol byte
  previews.
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
  one-byte, two-byte, three-byte, four-byte, five-byte, six-byte, seven-byte,
  and eight-byte packed reserved-suffix decode and encode,
  non-byte-aligned middle `UIntN` plus `ReservedBits(width, value)` plus
  `UIntN` decode and encode, including the narrow two-byte byte-interleaved
  middle layout,
  one-byte, two-byte, three-byte, four-byte, five-byte, six-byte, seven-byte,
  and eight-byte
  reserved prefix groups followed by two visible `UIntN` fields, including
  two-byte reserved prefix widths one through fourteen, three-byte reserved
  prefix widths seventeen through twenty-three, four-byte reserved prefix
  widths twenty-five through thirty-one, five-byte reserved prefix width
  thirty-three, six-byte reserved prefix width forty-one, seven-byte reserved
  prefix width forty-nine, and eight-byte reserved prefix width fifty-seven,
  and
  consecutive non-byte-aligned
  `UIntN` and
  `ReservedBits(width, value)` groups that complete one byte or one
  two-byte, three-byte, four-byte, five-byte, six-byte, seven-byte, or
  eight-byte big-endian storage unit
  ([Binary Schema Split Reserved Groups](../reference/implemented-proposals/binary-schema-split-reserved-groups.md),
  [Binary Schema General Reserved Bitfield Layouts](../reference/implemented-proposals/binary-schema-general-reserved-bitfield-layouts.md)),
  two-byte suffix groups where two visible `UIntN` fields, the second one
  `UInt8`, are followed by a non-byte-aligned
  `ReservedBits(width, value)` field
  ([Binary Schema Suffix Reserved Groups](../reference/implemented-proposals/binary-schema-suffix-reserved-groups.md)),
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
  inverse converters through written imports, unqualified public imports, and
  public function aliases, and direct or nested ADT constructor mapped encode
  projections for supported schema-local fields plus record-payload
  constructor slices, direction-specific nested dispatch payload decode helper
  eligibility with encode-helper diagnostics preserved for encode paths,
  standalone visible `UInt1` through `UInt7` decode and encode,
  visible-only packed `UInt1` through `UInt7` one-byte, two-byte,
  three-byte, four-byte, five-byte, six-byte, and seven-byte group decode
  and encode,
  bounded `Repeat(count_field, Payload)` primitive, same-module nested schema
  field, and public imported nested schema field decode and encode slices,
  bounded `Repeat(left_count - right_count, Payload)`,
  `Repeat(left_count + right_count, Payload)`, and
  `Repeat(left_count * right_count, Payload)` decode and encode with
  count-mismatch, invalid-count, and derived codec boundary coverage, bounded
  `Repeat(left_count / right_count, Payload)` decode and encode plus
  division-by-zero and derived codec boundary coverage,
  bounded `Repeat(count_field, ByteView(length_field))` decode and encode plus
  derived codec boundary slices, bounded
  `Repeat(count_field, ByteView(left_length + right_length))` decode and
  encode, length-bounded
  `ByteView(length_field)`, `ByteView(left_length - right_length)`, and
  `ByteView(left_length + right_length)`, and
  `ByteView(left_length * right_length)`, and
  `ByteView(left_length / right_length)` decode and encode,
  schema-owned `ByteView` payload multiple validation for earlier decoded
  `Int` fields and positive integer literals
  ([Binary Schema ByteView Payload Multiple](../reference/implemented-proposals/binary-schema-byteview-payload-multiple.md)),
  nested dispatch payload helpers for the one-bit reserved prefix
  `ReservedBits(1, 0)` followed by `UInt8`
  ([Binary Schema Dispatch One-Bit Reserved Payload Helpers](../reference/implemented-proposals/binary-schema-dispatch-one-bit-reserved-payload-helpers.md)),
  reserved byte prefixes `ReservedBits(2, 0)` and
  `ReservedBits(9, 0)` followed by `UInt8`
  ([Binary Schema Dispatch Reserved Byte Prefix Payload Helpers](../reference/implemented-proposals/binary-schema-dispatch-reserved-byte-prefix-payload-helpers.md)),
  declaration-time missing, forward, and wrong-role schema-local field
  reference diagnostics for repeat count fields and count expressions,
  byte-view lengths, byte-view payload multiple operands, dispatch tags, and
  extension-dispatch tags and lengths,
  schema-level structural validation for decoded `Int` fields,
  visible fixed exact-width field mismatch diagnostics for generated schema
  decode helpers, exact-width primitive encode, the HTTP/2 GOAWAY payload
  schema encode boundary, and narrow closed-dispatch and extension-dispatch
  primitive payload helpers plus eligible nested payload helpers.
  The nested payload helper slices accept eligible nested payload schemas that
  use the same generated binary schema helper path used for ordinary generated
  schema fields, including supported representation-only reserved-bit layouts,
  length-bounded `ByteView(length_field)` fields, additive
  `ByteView(left_length + right_length)` fields, subtractive
  `ByteView(left_length - right_length)` fields, product-sized
  `ByteView(left_length * right_length)` fields, quotient-sized
  `ByteView(left_length / right_length)` fields, and checked non-HTTP coverage
  combines the implemented helper vocabulary in one
  decode-and-encode schema. Closed dispatch payload cases
  with mixed primitive and nested decoded shapes are implemented for selected
  mappings keyed by the dispatch tag field when those mappings cover the
  dispatch cases and resolve to one target record shape. Same-module and
  public imported recursive closed-dispatch and extension-dispatch payload
  decode and encode support is implemented for the length-bounded forms when
  selected mappings cover every known case, resolve to one target record
  shape, and include a non-recursive base case; same-module wrapper
  dispatches may also select a separate eligible recursive payload schema
  through that same helper path. Earlier same-module recursive payload schemas
  and public imported recursive payload schemas are also accepted for
  decode-only length-bounded parent dispatch fields without selected mappings
  when the payload schema already has bounded recursive helper support and the
  parent includes a non-recursive primitive case
  ([Binary Schema Same-Module Recursive Dispatch Decode-Only](../reference/implemented-proposals/binary-schema-same-module-recursive-dispatch-decode-only.md)).
  Focused dispatch payload diagnostics now also
  name the failed recursive-helper fact for recursive payload rejections and
  name the generated decode and encode helper boundaries
  for resolved binary nested payload schemas that cannot expose those helpers,
  including unsupported `ByteView` payload layouts whose length field is not
  an earlier decoded `Int` field and unsupported representation-only
  `ReservedBits` payload layouts, and mapped payload schemas that decode but
  cannot project their mapping assignment back to schema-local fields for
  generated encode. The completed dispatch payload helper boundary diagnostics
  slice is archived under
  [Binary Schema Dispatch Payload Helper Boundary Diagnostics](../reference/implemented-proposals/binary-schema-dispatch-payload-helper-boundary-diagnostics.md).
  The completed direction-specific nested dispatch payload helper slice is
  archived under
  [Binary Schema Directional Dispatch Payload Helpers](../reference/implemented-proposals/binary-schema-directional-dispatch-payload-helpers.md).
  The completed nested dispatch
  `ByteView(length_field)` payload helper slice is archived under
  [Binary Schema Dispatch ByteView Payload Helpers](../reference/implemented-proposals/binary-schema-dispatch-byteview-payload-helpers.md).
  The completed nested dispatch
  `ByteView(left_length + right_length)` payload helper slice is archived under
  [Binary Schema Dispatch ByteView Add Payload Helpers](../reference/implemented-proposals/binary-schema-dispatch-byteview-add-payload-helpers.md).
  The completed nested dispatch
  `ByteView(left_length - right_length)` payload helper slice is archived under
  [Binary Schema Dispatch ByteView Subtract Payload Helpers](../reference/implemented-proposals/binary-schema-dispatch-byteview-subtract-payload-helpers.md).
  The completed nested dispatch
  `ByteView(left_length * right_length)` payload helper slice is archived under
  [Binary Schema Dispatch ByteView Product Payload Helpers](../reference/implemented-proposals/binary-schema-dispatch-byteview-product-payload-helpers.md).
  The completed nested dispatch
  `ByteView(left_length / right_length)` payload helper slice is archived under
  [Binary Schema Dispatch ByteView Quotient Payload Helpers](../reference/implemented-proposals/binary-schema-dispatch-byteview-quotient-payload-helpers.md).
  The completed bounded repeat helper binding slice is archived under
  [Binary Schema Repeat Helper Bindings](../reference/implemented-proposals/binary-schema-repeat-schema-payload-helpers.md).
  The completed narrow arithmetic mapped encode slice is archived under
  [Binary Schema Mapping Arithmetic Encode](../reference/implemented-proposals/binary-schema-mapping-arithmetic-encode.md).
  The completed mapped encode projection diagnostic slice is archived under
  [Binary Schema Mapped Encode Projection Diagnostics](../reference/implemented-proposals/binary-schema-mapped-encode-projection-diagnostics.md).
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
  The completed opt-in reserved-bit mapping exposure slice is archived under
  [Binary Schema Reserved Bit Mapping Exposure](../reference/implemented-proposals/binary-schema-reserved-bit-mapping-exposure.md).
  The completed one-byte reserved suffix slice is archived under
  [Binary Schema One-Byte Reserved Suffix](../reference/implemented-proposals/binary-schema-one-byte-reserved-suffix.md).
  The completed `UInt8` plus multi-byte reserved suffix slice is archived
  under
  [Binary Schema Byte-Visible Reserved Suffix](../reference/implemented-proposals/binary-schema-byte-visible-reserved-suffix.md).
  The completed `ReservedBits(15, value)` followed by `UInt1` two-field
  boundary is archived under
  [Binary Schema Reserved Fifteen-Bit Prefix](../reference/implemented-proposals/binary-schema-reserved-fifteen-bit-prefix.md).
  The completed visible-only packed two-byte group slice is archived under
  [Binary Schema Packed Visible Two-Byte Groups](../reference/implemented-proposals/binary-schema-packed-visible-two-byte-groups.md).
  The completed visible-only packed three-byte group slice is archived under
  [Binary Schema Packed Visible Three-Byte Groups](../reference/implemented-proposals/binary-schema-packed-visible-three-byte-groups.md).
  The completed visible-only packed four-byte group slice is archived under
  [Binary Schema Packed Visible Four-Byte Groups](../reference/implemented-proposals/binary-schema-packed-visible-four-byte-groups.md).
  The completed visible-only packed five-byte group slice is archived under
  [Binary Schema Packed Visible Five-Byte Groups](../reference/implemented-proposals/binary-schema-packed-visible-five-byte-groups.md).
  The completed visible-only packed six-byte group slice is archived under
  [Binary Schema Packed Visible Six-Byte Groups](../reference/implemented-proposals/binary-schema-packed-visible-six-byte-groups.md).
  The completed visible-only packed seven-byte group slice is archived under
  [Binary Schema Packed Visible Seven-Byte Groups](../reference/implemented-proposals/binary-schema-packed-visible-seven-byte-groups.md).
  The completed six-byte reserved suffix slice is archived under
  [Binary Schema Six-Byte Reserved Suffix](../reference/implemented-proposals/binary-schema-six-byte-reserved-suffix.md).
  The completed seven-byte and eight-byte reserved suffix slice is archived
  under
  [Binary Schema Wide Reserved Suffix Groups](../reference/implemented-proposals/binary-schema-wide-reserved-suffix-groups.md).
  The completed seven-byte and eight-byte reserved prefix group slice is
  archived under
  [Binary Schema Wide Reserved Prefix Groups](../reference/implemented-proposals/binary-schema-wide-reserved-prefix-groups.md).
- [Codec Execution Boundary](codec-execution-boundary.md): define remaining
  executable decode and encode behavior beyond the implemented codec
  declaration source-surface slice, decode function signature boundary,
  mapped decode value boundary, encode function return and mapped value
  parameter boundaries, derived codec mapping value boundary checks,
  source-visible decode and encode result vocabulary, direct source-visible
  `DecodeError`, `DecodeErrorWithReason`, and `EncodeError` command-facing
  projection from run entry result failures, generated binary schema
  decode-step helper slice for implemented exact-width, middle reserved,
  repeat-backed, and same-module and public imported nested dispatch payload
  boundaries,
  hand-written codec decode consumed-count validation, hand-written codec
  encode and decode execution boundaries including caller-owned parser-state
  retention around `Decoded` and `NeedMore`, the bounded `ByteView` plus base
  `ByteOffset` hand-written decode example with non-consuming short-input
  readiness, same-module hand-written `NeedEnd` readiness preservation and
  closed-input projection, and absolute malformed-input offsets,
  source-visible partial encode preservation and same-module plus imported
  resume, plus eligible derived codec decode and
  encode execution boundaries, including budgeted derived encode, over the
  checked non-HTTP composite helper shape and general generated helper shape,
  additive, subtractive, quotient-sized, and product-sized `ByteView` payload
  fields, arithmetic-count and quotient-count
  repeated primitive fields, same-module recursive closed and extension
  dispatch payload helpers, byte-aligned representation-only
  `ReservedBits(width, value)` fields through the derived decode boundary,
  derived bounded `ByteView` plus explicit base-offset decode projection,
  standalone visible `UInt1` through `UInt7`
  fields, visible-only packed two-byte, three-byte, four-byte, five-byte,
  six-byte, and seven-byte groups,
  opt-in visible flag bitset fields, including generated-helper-backed
  `Flag24be` and `Flag24le` fields, wide reserved suffix groups, wide reserved
  prefix groups, the narrow `ReservedBits(9, 0)` plus `UInt8` two-byte prefix
  helper route,
  schema mappings that call pure same-module or imported public converters
  with one or more supported structural arguments through generated decode
  mapping and the derived codec decode boundary,
  and selected structural mapping encode slice,
  and derived helper eligibility diagnostics for unsupported generated decode
  and encode directions.
  The completed same-module hand-written encode resume slice is archived under
  [Codec Hand-Written Encode Resume](../reference/implemented-proposals/codec-hand-written-encode-resume.md).
  The completed same-module hand-written `NeedEnd` readiness preservation
  slice is archived under
  [Codec Hand-Written NeedEnd Boundary](../reference/implemented-proposals/codec-hand-written-need-end-boundary.md).
  The completed imported hand-written codec boundary is archived under
  [Codec Imported Hand-Written Boundary](../reference/implemented-proposals/codec-imported-hand-written-boundary.md).
  The completed imported derived codec boundary is archived under
  [Codec Imported Derived Boundary](../reference/implemented-proposals/codec-imported-derived-boundary.md).
- [Schema And Protocol Diagnostics](schema-and-protocol-diagnostics.md):
  define remaining structured diagnostics beyond the implemented closed-input
  `ByteView` read truncation, schema fixed-field mismatch, frame-header schema
  truncation, reserved-bit mismatch, payload length boundary, field-local
  schema validation details, structured schema byte previews, and the HTTP/2
  client connection preface failures, frame-size peer-limits with
  protocol-owned frame-header byte previews, header-list-size peer-limits,
  flow-control peer-limits with protocol-owned DATA payload byte previews,
  SETTINGS value range peer-limit, stream id domain failures with
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
  direct source-visible `DecodeError`, `DecodeErrorWithReason`, and
  `EncodeError` result failures with JSON and human fixtures,
  command-facing projection for
  `DecodeStep::Invalid(DecodeError(...))`,
  `DecodeStep::Invalid(DecodeErrorWithReason(...))`, and
  `DecodeStep::NeedMore(...)` entry results, reason-carrying hand-written
  codec invalid-input decode projection with optional carried byte-helper
  context, codec-owned hand-written invalid-input ids including
  `codec.packet_kind_invalid` direct and `DecodeStep::Invalid(...)`
  command projections, codec-owned checksum mismatch projection with
  expected checksum, actual checksum, and failure
  reason details, codec-owned length mismatch projection with expected
  length, actual length, and failure reason details, generated binary
  schema decode integer range failures, generated bounded repeated schema
  field truncation diagnostics with indexed field paths in JSON and human
  output, plus hand-written codec decode consumed-count failures and their
  command-facing projection.
  The completed codec-owned decode invalid id slice is archived under
  [Codec Owned Decode Invalid Id Diagnostics](../reference/implemented-proposals/codec-owned-decode-invalid-id-diagnostics.md).
- [HTTP/2 Sans-I/O Protocol Core](http2-sans-io-protocol-core.md): define the
  remaining concrete pure protocol-core behavior beyond the implemented
  ordinary-source receive-state, diagnostics, settings, stream lifecycle,
  HPACK behavior beyond the checked fixture boundary,
  unknown extension-frame, receive flow-control, send-intent,
  `RST_STREAM`, PRIORITY including idle-stream receive while another stream is
  tracked open and half-closed-local receive, HEADERS with the PRIORITY flag,
  PING, GOAWAY, local
  GOAWAY outbound HEADERS boundary, server-side `PUSH_PROMISE` rejection,
  server-side outbound
  `PUSH_PROMISE` send-intent, client-side `PUSH_PROMISE` receive and
  promised response HEADERS admission including local disable-push receive
  policy, and
  half-closed-by-peer outbound DATA send-intent slices,
  request-side, response-side, request-trailer, and response-trailer
  header-list validation,
  the source-visible request `:path` value rule, the source-visible `te`
  header value rule, the `content-length`
  header-list consistency slice, accepted `content-length` body-length
  accounting for tracked inbound and outbound DATA,
  the malformed Huffman padding fixture diagnostic, and the outbound HPACK
  fixture header-list encoder slice, including static-name literal fixtures,
  visible-ASCII ordinary new-name literal-without-indexing for outbound
  HEADERS and `PUSH_PROMISE`,
  full-table single-byte Huffman-marked string literal decoding and encoding,
  and the bounded
  stateful dynamic-table fixture encoder path across outbound HEADERS and
  server-side `PUSH_PROMISE`, focused unsupported-Huffman EOS
  and non-visible decoded-byte diagnostics, focused malformed string-length
  and raw string value fixture diagnostics, general visible-ASCII raw literal
  values, raw field-name validation through header-list diagnostics, inbound
  fixture dynamic-table insertion, ordinary raw new-name dynamic-indexed reuse
  and eviction, raw new-name literal-never-indexed receive without dynamic
  insertion, focused dynamic-index lookup failure diagnostics, checked
  dynamic-name continuation diagnostics, checked
  inbound table-size update placement, malformed-integer, and trailing-byte
  diagnostics, checked outbound
  dynamic table-size update encoding and state handoff into later HEADERS
  and server-side `PUSH_PROMISE`, received peer
  `SETTINGS_HEADER_TABLE_SIZE` values driving later outbound HPACK fixture
  capacity, checked outbound dynamic-name literal fixture encoding, and
  checked outbound `PUSH_PROMISE` rejection after peer
  `SETTINGS_ENABLE_PUSH = 0`, plus deterministic
  `hpack-bytes-*` multi-byte non-visible Huffman fixture labels,
  recorded under `../specification/` and
  `../reference/implemented-proposals/`. Planned work still includes
  broader protocol-core behavior and full HPACK behavior beyond the checked
  fixture boundary, including full HPACK compression and unbounded
  dynamic-table behavior.
  The completed half-closed-by-peer outbound DATA send-intent slice is archived
  under
  [HTTP/2 Half-Closed-By-Peer Outbound DATA](../reference/implemented-proposals/http2-half-closed-by-peer-outbound-data.md).
  The completed half-closed-local PRIORITY receive slice is archived under
  [HTTP/2 Half-Closed-Local PRIORITY Receive](../reference/implemented-proposals/http2-half-closed-local-priority-receive.md).
  The completed outbound DATA send-credit refill from peer `WINDOW_UPDATE` and
  `SETTINGS_INITIAL_WINDOW_SIZE` delta slices are covered by
  [HTTP/2 Outbound DATA Flow Control](../reference/implemented-proposals/http2-outbound-data-flow-control.md).
  The completed outbound DATA post-GOAWAY send-intent boundary is archived
  under
  [HTTP/2 Outbound DATA GOAWAY Boundary](../reference/implemented-proposals/http2-outbound-data-goaway-boundary.md).
  The completed GOAWAY receive lifecycle slice is archived under
  [HTTP/2 GOAWAY Receive Lifecycle](../reference/implemented-proposals/http2-goaway-receive-lifecycle.md).
  The completed accepted `content-length` body accounting slices are archived
  under
  [HTTP/2 Content-Length Body Accounting](../reference/implemented-proposals/http2-content-length-body-accounting.md).
  The completed dynamic-name continuation diagnostic slice is archived under
  [HTTP/2 HPACK Dynamic Name Continuation Diagnostics](../reference/implemented-proposals/http2-hpack-dynamic-name-continuation-diagnostics.md).
  The completed outbound dynamic-name literal fixture slice is archived under
  [HTTP/2 Outbound HPACK Dynamic-Name Literal](../reference/implemented-proposals/http2-outbound-hpack-dynamic-name-literal.md).
  The completed outbound `PUSH_PROMISE` peer enable-push setting slice is
  archived under
  [HTTP/2 Outbound PUSH_PROMISE Enable-Push Setting](../reference/implemented-proposals/http2-outbound-push-promise-enable-push-setting.md).
  The completed inbound dynamic-table fixture slice is archived under
  [HTTP/2 HPACK Dynamic Table Fixture](../reference/implemented-proposals/http2-hpack-dynamic-table-eviction-fixture.md).
- [Network Effect Integration Boundary](network-effect-integration-boundary.md):
  define remaining transport adapter, richer production socket APIs, richer
  stream-routing, richer deadline, cancellation, channel, and task behavior
  beyond the implemented transport, bounded channel-first routing, general
  receiver-list routing, task, relative and absolute monotonic deadline,
  cancellation, deadline-aware listener
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
  `net::write_chunks`, adapter-owned outbound write-failure boundary,
  production-loopback listen, sequential accept, read, write, clean listener
  end, close lifecycle, two-stream adapter handler/action lifecycle, and
  listener-drain adapter lifecycle, listener-drain read-failure runtime
  boundary, deadline-aware adapter lifecycle, deadline-aware accept and read
  failure runtime boundaries, production cancellable deadline-aware adapter
  lifecycle and outcome boundary, adapter close-failure runtime boundary, and
  explicit listener-close boundary, adapter-owned cancellation owner
  lifecycle boundary, production owner-drain cancellable deadline lifecycle
  boundary, cancellation owner status query, absolute monotonic deadline
  construction
  ([Network Deadline At Boundary](../reference/implemented-proposals/network-deadline-at-boundary.md)),
  general
  receiver-list channel-first routing through the
  current checked select-many boundary including stale route-count fixture
  cleanup, receiver-list cancellable channel-first routing,
  timeout-result selection, receiver-list cancellable timeout-result
  selection, two-receiver timeout-result selection, and two-receiver
  cancellable timeout-result selection slices
  documented under `../specification/`, including generated binary schema
  decode-step invalid-input projection at explicit absolute offsets; completed
  proposal records live under `../reference/implemented-proposals/`.
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
  The completed source-visible monotonic clock boundary is archived under
  [Network Monotonic Clock Boundary](../reference/implemented-proposals/network-monotonic-clock-boundary.md).
  The completed write-side stream half-close boundary is archived under
  [Network Stream Shutdown Write Boundary](../reference/implemented-proposals/network-stream-shutdown-write-boundary.md).
  The completed explicit listener-close boundary is archived under
  [Network Listener Close Boundary](../reference/implemented-proposals/network-listener-close-boundary.md).
  The completed adapter-owned multi-handler outbound write-ordering and
  outbound write-failure slices are archived under
  [Network Adapter Outbound Write Ordering](../reference/implemented-proposals/network-adapter-outbound-write-ordering.md).
  The completed adapter-owned clean shutdown slice is archived under
  [Network Adapter Clean Shutdown](../reference/implemented-proposals/network-adapter-clean-shutdown.md).
  The completed adapter-owned cancellation owner slice is archived under
  [Network Cancel Owner Boundary](../reference/implemented-proposals/network-cancel-owner-boundary.md).
  The completed cancellation owner status query slice is archived under
  [Network Cancel Owner Status](../reference/implemented-proposals/network-cancel-owner-status.md).
  The completed production owner-drain cancellable deadline lifecycle slice is
  archived under
  [Network Production Owner-Drain Lifecycle](../reference/implemented-proposals/network-production-owner-drain-lifecycle.md).
  The completed receiver-list select-many routing and stale route-count
  fixture cleanup slice is archived under
  [Network Channel Select-Many Routing](../reference/implemented-proposals/network-channel-select-many-routing.md).

## Update When

- New proposal work is added, split, superseded, completed, or removed.
- Proposal work becomes implemented and the resulting behavior is documented
  under `../specification/`.
- A completed proposal record moves to
  `../reference/implemented-proposals/`.
