# Proposals

This directory catalogs planned or accepted work that is not fully documented
as current behavior under `../specification/`. Proposal text is not current
language behavior unless the matching specification page also states it.

Use this page as a catalog only. Pick the proposal that matches the task, then
compare it with `../specification/` before changing behavior.

## Catalog

- [Local Inference And Annotation Elision](local-inference-and-annotation-elision.md):
  define remaining annotation-elision work for callback inputs outside the
  compiler-known, concrete declared-helper signature including visible public
  function aliases, concrete record-field expected-type, concrete
  local-binding expected-type, and concrete
  direct return-position expected-type, concrete match-arm expected-type, and
  concrete if-branch expected-type, concrete constructor-payload
  expected-type, concrete collection element expected-type, and concrete
  dictionary value expected-type paths, and
  other paths beyond the implemented same-function local `let`, non-empty
  collection initializer, `if` branch local `let`, empty collection expected-type,
  local `let` expected-type path, nested initializer expected-type
  propagation, hole expected-type flow, empty collection callback return,
  payload-carrying ADT constructor inference, match scrutinee
  constructor-pattern inference, local pattern `let` inference,
  compiler-known prelude callback argument including `vec_try_map_with`,
  dictionary callback alias, declared helper callback argument, effectful
  declared-helper callback, source-backed prelude callback fallback, declared
  helper callback alias, record-field callback, local callback binding, local
  callback binding annotation elision,
  direct return callback, match-arm callback, if-branch callback, callback
  return expected-type,
  constructor-payload callback, variadic declared-helper callback parameter,
  collection callback element, dictionary value callback, examples cleanup,
  and diagnostic-details slices specified in `../specification/types.md` and
  `../specification/diagnostics-json.md`.
  The completed private helper call-site inference, prelude callback argument
  inference including `vec_try_map_with`, dictionary callback alias inference,
  declared helper callback argument inference, effectful declared-helper
  callback inference, declared helper callback alias inference, source-backed
  prelude callback fallback, record-field callback inference, local callback
  binding inference, local callback binding
  annotation-elision inference, direct return callback inference, callback
  return expected-type inference, constructor-payload callback inference,
  collection callback element inference, non-empty
  collection initializer inference, ADT constructor payload inference, match
  scrutinee constructor-pattern inference, local pattern `let` inference,
  `if` branch local `let` inference,
  local `let` expected-type path inference,
  nested initializer expected-type propagation, hole expected-type flow, and
  examples cleanup, and diagnostic-details slices are archived under
  [local-inference-private-helper-call-site.md](../reference/implemented-proposals/local-inference-private-helper-call-site.md),
  [local-inference-prelude-callback-argument.md](../reference/implemented-proposals/local-inference-prelude-callback-argument.md),
  [local-inference-dictionary-callback-aliases.md](../reference/implemented-proposals/local-inference-dictionary-callback-aliases.md),
  [local-inference-declared-helper-callback-argument.md](../reference/implemented-proposals/local-inference-declared-helper-callback-argument.md),
  [local-inference-effectful-declared-helper-callback.md](../reference/implemented-proposals/local-inference-effectful-declared-helper-callback.md),
  [local-inference-declared-helper-callback-alias.md](../reference/implemented-proposals/local-inference-declared-helper-callback-alias.md),
  [local-inference-prelude-callback-fallback.md](../reference/implemented-proposals/local-inference-prelude-callback-fallback.md),
  [local-inference-record-field-callback.md](../reference/implemented-proposals/local-inference-record-field-callback.md),
  [local-inference-local-callback-binding.md](../reference/implemented-proposals/local-inference-local-callback-binding.md),
  [local-inference-local-callback-binding-annotation-elision.md](../reference/implemented-proposals/local-inference-local-callback-binding-annotation-elision.md),
  [local-inference-direct-return-callback.md](../reference/implemented-proposals/local-inference-direct-return-callback.md),
  [local-inference-match-arm-callback.md](../reference/implemented-proposals/local-inference-match-arm-callback.md),
  [local-inference-if-branch-callback.md](../reference/implemented-proposals/local-inference-if-branch-callback.md),
  [local-inference-callback-return-expected-type.md](../reference/implemented-proposals/local-inference-callback-return-expected-type.md),
  [local-inference-constructor-payload-callback.md](../reference/implemented-proposals/local-inference-constructor-payload-callback.md),
  [local-inference-collection-callback-element.md](../reference/implemented-proposals/local-inference-collection-callback-element.md),
  [local-inference-dictionary-value-callback.md](../reference/implemented-proposals/local-inference-dictionary-value-callback.md),
  [local-inference-variadic-callback-parameter.md](../reference/implemented-proposals/local-inference-variadic-callback-parameter.md),
  [local-inference-non-empty-collection-initializer.md](../reference/implemented-proposals/local-inference-non-empty-collection-initializer.md),
  [local-inference-adt-constructor-payload.md](../reference/implemented-proposals/local-inference-adt-constructor-payload.md),
  [local-inference-match-scrutinee-constructor-pattern.md](../reference/implemented-proposals/local-inference-match-scrutinee-constructor-pattern.md),
  [local-inference-local-pattern-let.md](../reference/implemented-proposals/local-inference-local-pattern-let.md),
  [local-inference-if-branch-local-let.md](../reference/implemented-proposals/local-inference-if-branch-local-let.md),
  [local-inference-local-let-expected-type-paths.md](../reference/implemented-proposals/local-inference-local-let-expected-type-paths.md),
  [local-inference-nested-initializer-expected-type.md](../reference/implemented-proposals/local-inference-nested-initializer-expected-type.md),
  [local-inference-hole-expected-type-flow.md](../reference/implemented-proposals/local-inference-hole-expected-type-flow.md),
  [local-inference-examples-cleanup.md](../reference/implemented-proposals/local-inference-examples-cleanup.md),
  and
  [local-inference-diagnostic-details.md](../reference/implemented-proposals/local-inference-diagnostic-details.md).
- [HTTP/2 Binary Schema Design Driver](http2-binary-schema-design-driver.md):
  use an HTTP/2 sans-I/O server core to drive binary schema, codec, and
  standard-library design.
- [Schema Declaration Surface](schema-declaration-surface.md): define only the
  remaining generated binary helper coverage and later schema-composition
  surfaces. Current schema syntax and helper behavior are specified under
  `../specification/`; the completed recursive format-neutral encode boundary
  and its evidence are archived under
  [Recursive Format-Neutral Schema Encode Shapes](../reference/implemented-proposals/recursive-format-neutral-schema-encode-shapes.md).
- [Binary Data Standard Library](binary-data-standard-library.md): define the
  remaining binary-buffer conversion and protocol-facing
  diagnostic behavior beyond the implemented byte vocabulary, byte-view, fixed
  big-endian and little-endian read/write through the current source-visible
  helper width set including `u56`, bounded view buffer helper,
  view-to-chunk materialization, outgoing chunk-list, stream-input, pending
  input and outgoing immutable chunk collection for protocol examples,
  budgeted outgoing whole-chunk production,
  byte-view freeze preservation across task and channel boundaries,
  schema-facing byte data conversion boundary,
  source-visible `ByteView` range diagnostics with byte previews, checked byte
  write conversion diagnostics, and schema byte-preview diagnostic slices plus
  HTTP/2 client preface, invalid frame-kind, and PRIORITY self-dependency
  protocol byte previews, HTTP/2 invalid stream-id domain protocol byte
  previews, plus the HPACK fixture unsupported-header-block,
  unsupported-static-index, and malformed-Huffman-padding protocol byte
  previews, HPACK static unsupported-index protocol byte preview, HPACK
  fixture Huffman EOS protocol byte preview, HTTP/2 SETTINGS value range
  protocol byte preview,
  HTTP/2 `WINDOW_UPDATE` invalid-increment protocol byte preview, HTTP/2 DATA
  receive flow-control protocol byte preview, HTTP/2 unexpected SETTINGS ACK
  protocol byte preview, HTTP/2 invalid DATA padding protocol byte preview,
  plus HTTP/2 frame-size,
  header-list, header-table, concurrent-stream receive-limit, and
  stream-after-GOAWAY protocol byte previews, and HTTP/2 request, response,
  request-trailer, and response-trailer header-list validation protocol byte
  previews.
  The outgoing chunk production slice is archived under
  [Binary Data Outgoing Chunk Production](../reference/implemented-proposals/binary-data-outgoing-chunk-production.md).
  The schema conversion boundary slice is archived under
  [Binary Data Schema Conversion Boundary](../reference/implemented-proposals/binary-data-schema-conversion-boundary.md).
  The HPACK Huffman EOS protocol-facing byte diagnostic slice is archived
  under
  [Binary Data HPACK Huffman EOS Diagnostic](../reference/implemented-proposals/binary-data-hpack-huffman-eos-diagnostic.md).
  The HPACK static index byte preview diagnostics slice is archived under
  [Binary Data HPACK Static Index Byte Preview Diagnostics](../reference/implemented-proposals/binary-data-hpack-static-index-byte-preview-diagnostics.md).
- [Binary Schema Primitives And Dispatch](binary-schema-primitives-and-dispatch.md):
  define remaining general binary schema primitive and dispatch behavior.
  Implemented slices include source-surface exact-width and
  `ReservedBits(width, value)` declarations, explicit `Http2FrameHeaderWire`
  decode used by the HTTP/2 protocol-core frame-header path,
  width-sample primitive decode, `UInt16le`, `UInt24le`,
  `UInt31le`, `UInt32le`, `UInt48le`, `UInt56le`, and `UInt64le`
  little-endian primitive decode and encode,
  `UInt56le` and `UInt64le` direct visible little-endian generated helper
  parity
  ([Binary Schema UInt56le And UInt64le Parity](../reference/implemented-proposals/binary-schema-u56le-u64le-parity.md)),
  `UInt16be`, `UInt24be`, `UInt31be`, `UInt32be`, `UInt56be`, and `UInt64be`
  direct visible
  big-endian generated helper parity
  ([Binary Schema Big-Endian Width Parity](../reference/implemented-proposals/binary-schema-big-endian-width-parity.md)),
  `UInt40be` and `UInt40le`
  five-byte primitive decode and encode
  ([Binary Schema UInt40 Primitives](../reference/implemented-proposals/binary-schema-u40-primitives.md)),
  `UInt48be` six-byte big-endian primitive decode and encode,
  byte-aligned reserved-bit decode and encode,
  general direct non-byte-aligned reserved prefixes before `UInt8` whose
  padded groups fit in at most eight big-endian bytes
  ([Binary Schema General Reserved Byte Prefixes](../reference/implemented-proposals/binary-schema-general-reserved-byte-prefixes.md)),
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
  direction-specific nested dispatch payload decode helper eligibility with
  encode-helper diagnostics preserved for encode paths,
  standalone visible `UInt1` through `UInt7` decode and encode,
  visible-only packed `UInt1` through `UInt7` one-byte, two-byte,
  three-byte, four-byte, five-byte, six-byte, seven-byte, and eight-byte
  group decode and encode,
  bounded `Repeat(count_field, Payload)` primitive, same-module nested schema
  field, public imported nested schema field, and same-module recursive nested
  schema field decode and encode slices,
  bounded `Repeat(left_count - right_count, Payload)`,
  `Repeat(left_count + right_count, Payload)`, and
  `Repeat(left_count * right_count, Payload)` decode and encode with
  count-mismatch, invalid-count, and compatibility derived codec boundary
  coverage, bounded
  `Repeat(left_count / right_count, Payload)` decode and encode plus
  division-by-zero and compatibility derived codec boundary coverage,
  bounded `Repeat(count_field, ByteView(length_field))` decode and encode plus
  compatibility derived codec boundary slices, bounded
  `Repeat(count_field, ByteView(left_length + right_length))` decode and
  encode, bounded
  `Repeat(count_field, ByteView(left_length - right_length))` decode,
  length-bounded
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
  direct lowercase dispatch payloads written as zero-reserved subbyte payloads
  from `uint1 reserves 0` through `uint7 reserves 0`
  ([Binary Schema Dispatch Lowercase Subbyte Reserved Payloads](../reference/implemented-proposals/binary-schema-dispatch-lowercase-subbyte-reserved-payloads.md)),
  direct lowercase dispatch payloads written as nonzero subbyte payloads from
  `uint1 reserves 1` through `uint7 reserves 127` when the reserved value fits
  the declared width
  ([Binary Schema Dispatch Nonzero Lowercase Subbyte Reserved Payloads](../reference/implemented-proposals/binary-schema-dispatch-nonzero-lowercase-subbyte-reserved-payloads.md)),
  schema-local field reference diagnostics for repeat count fields and count
  expressions, byte-view lengths, byte-view payload multiple operands,
  dispatch tags, and extension-dispatch tags and lengths
  ([Binary Schema Field Reference Diagnostics](../reference/implemented-proposals/binary-schema-field-reference-diagnostics.md)),
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
  with mixed primitive and nested decoded shapes are implemented for
  schema-local visible record helper paths. Same-module and public imported
  recursive closed-dispatch and extension-dispatch payload decode and encode
  support is implemented for the length-bounded forms when the helper path has
  a non-recursive base case; same-module wrapper dispatches may also select a
  separate eligible recursive payload schema through that same helper path.
  Same-module recursive payload helpers expose the finite primitive payload
  shape for length-bounded recursive fields with a non-recursive primitive
  base case
  ([Binary Schema Same-Module Recursive Dispatch Helpers](../reference/implemented-proposals/binary-schema-same-module-recursive-dispatch-helpers.md)).
  Focused dispatch payload diagnostics now also
  name the failed recursive-helper fact for recursive payload rejections and
  name the generated decode and encode helper boundaries
  for resolved binary nested payload schemas that cannot expose those helpers,
  including unsupported `ByteView` payload layouts whose length field is not
  an earlier decoded `Int` field and unsupported representation-only
  `ReservedBits` payload layouts. The completed dispatch payload helper boundary diagnostics
  slice is archived under
  [Binary Schema Dispatch Payload Helper Boundary Diagnostics](../reference/implemented-proposals/binary-schema-dispatch-payload-helper-boundary-diagnostics.md).
  The completed binary schema anonymous record decode slice is archived under
  [Binary Schema Anonymous Record Decode](../reference/implemented-proposals/binary-schema-anonymous-record-decode.md).
  The completed binary schema anonymous record encode slice is archived under
  [Binary Schema Anonymous Record Encode](../reference/implemented-proposals/binary-schema-anonymous-record-encode.md).
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
  The completed public imported nested dispatch
  `ByteView(left_length / right_length)` payload helper slice is archived under
  [Binary Schema Imported Dispatch ByteView Quotient Payload Helpers](../reference/implemented-proposals/binary-schema-imported-dispatch-byteview-quotient-payload-helpers.md).
  The completed bounded repeat helper binding slice, including
  representation-only lowercase reserved repeat payloads and same-module
  recursive repeated nested payload helpers, is archived under
  [Binary Schema Repeat Helper Bindings](../reference/implemented-proposals/binary-schema-repeat-schema-payload-helpers.md).
  The completed bounded repeat `ByteView(left_length - right_length)` payload
  helper slice is archived under
  [Binary Schema Repeat ByteView Subtract Helpers](../reference/implemented-proposals/binary-schema-repeat-byteview-subtract-helpers.md).
  The completed `UInt40be` and
  `UInt40le` exact-width primitive slice is archived under
  [Binary Schema UInt40 Primitives](../reference/implemented-proposals/binary-schema-u40-primitives.md).
  The completed `UInt48be` and
  `UInt48le` exact-width primitive slice is archived under
  [Binary Schema UInt48 Primitives](../reference/implemented-proposals/binary-schema-u48-primitives.md).
  The completed `UInt56be` and
  `UInt56le` exact-width primitive slice is archived under
  [Binary Schema UInt56 Primitives](../reference/implemented-proposals/binary-schema-u56-primitives.md).
  The completed reserved-byte-prefix encode slice for `ReservedBits(2, 0)`
  and `ReservedBits(9, 0)` followed by `UInt8` is archived under
  [Binary Schema Reserved Byte Prefix Encode](../reference/implemented-proposals/binary-schema-reserved-byte-prefix-encode.md).
  The completed general direct reserved-byte-prefix rule is archived under
  [Binary Schema General Reserved Byte Prefixes](../reference/implemented-proposals/binary-schema-general-reserved-byte-prefixes.md).
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
  The completed visible-only packed eight-byte group slice is archived under
  [Binary Schema Packed Visible Eight-Byte Groups](../reference/implemented-proposals/binary-schema-packed-visible-eight-byte-groups.md).
  The completed six-byte reserved suffix slice is archived under
  [Binary Schema Six-Byte Reserved Suffix](../reference/implemented-proposals/binary-schema-six-byte-reserved-suffix.md).
  The completed seven-byte and eight-byte reserved suffix slice is archived
  under
  [Binary Schema Wide Reserved Suffix Groups](../reference/implemented-proposals/binary-schema-wide-reserved-suffix-groups.md).
  The completed seven-byte and eight-byte reserved prefix group slice is
  archived under
  [Binary Schema Wide Reserved Prefix Groups](../reference/implemented-proposals/binary-schema-wide-reserved-prefix-groups.md).
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
  binary schema encode value-representation failures, generated schema
  dispatch `EncodeError` command-facing projection for unknown tag, length
  mismatch, and tag/payload mismatch failures, command-facing projection
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
  length, actual length, and failure reason details, codec-owned payload
  length mismatch projection with expected payload length, actual payload
  length, and failure reason details, codec-owned padding mismatch projection
  with expected padding length, actual padding length, and failure reason
  details, codec-owned integer out-of-range projection with byte width,
  accepted integer range, actual decoded value, and failure reason details,
  codec-owned sequence mismatch projection with
  expected sequence, actual sequence, and failure reason details,
  codec-owned version mismatch projection with expected
  version, actual version, and failure reason details, codec-owned tag
  mismatch projection with expected tag, actual tag, and failure reason
  details, codec-owned magic mismatch projection with expected magic, actual
  magic, and failure reason details, codec-owned unsupported feature
  projection with unsupported feature and failure reason details, generated
  binary schema decode
  integer range failures,
  generated bounded repeated schema
  field truncation diagnostics with indexed field paths in JSON and human
  output.
  The completed codec-owned decode invalid id slice is archived under
  [Codec Owned Decode Invalid Id Diagnostics](../reference/implemented-proposals/codec-owned-decode-invalid-id-diagnostics.md).
  The completed codec-owned consumed-count invalid slice is archived under
  [Codec Consumed Count Invalid Diagnostics](../reference/implemented-proposals/codec-consumed-count-invalid-diagnostics.md).
  The completed codec-owned sequence mismatch slice is archived under
  [Codec Sequence Mismatch Diagnostics](../reference/implemented-proposals/codec-sequence-mismatch-diagnostics.md).
  The completed codec-owned payload length mismatch slice is archived under
  [Codec Payload Length Mismatch Diagnostics](../reference/implemented-proposals/codec-payload-length-mismatch-diagnostics.md).
  The completed codec-owned padding mismatch slice is archived under
  [Codec Padding Mismatch Diagnostics](../reference/implemented-proposals/codec-padding-mismatch-diagnostics.md).
  The completed codec-owned integer out-of-range slice is archived under
  [Codec Integer Out-Of-Range Diagnostics](../reference/implemented-proposals/codec-integer-out-of-range-diagnostics.md).
  The completed codec-owned version mismatch slice is archived under
  [Codec Version Mismatch Diagnostics](../reference/implemented-proposals/codec-version-mismatch-diagnostics.md).
  The completed codec-owned tag mismatch slice is archived under
  [Codec Tag Mismatch Diagnostics](../reference/implemented-proposals/codec-tag-mismatch-diagnostics.md).
  The completed codec-owned magic mismatch slice is archived under
  [Codec Magic Mismatch Diagnostics](../reference/implemented-proposals/codec-magic-mismatch-diagnostics.md).
  The completed codec-owned unsupported feature slice is archived under
  [Codec Unsupported Feature Diagnostics](../reference/implemented-proposals/codec-unsupported-feature-diagnostics.md).
- [HTTP/2 Sans-I/O Protocol Core](http2-sans-io-protocol-core.md): define the
  remaining concrete pure protocol-core behavior beyond the implemented
  ordinary-source receive-state, diagnostics, settings, list-backed stream
  lifecycle, HPACK behavior beyond the checked fixture boundary,
  unknown extension-frame, receive flow-control, send-intent,
  `RST_STREAM`, HEADERS with the PRIORITY flag, GOAWAY, local
  GOAWAY outbound HEADERS boundary, client-side outbound HEADERS local-stream
  admission and retained stream-id ordering,
  server-side `PUSH_PROMISE` rejection,
  outbound `PRIORITY` post-GOAWAY send-intent boundary,
  server-side outbound
  `PUSH_PROMISE` send-intent, client-side `PUSH_PROMISE` receive and
  promised response HEADERS admission including local disable-push receive
  policy, and half-closed-by-peer outbound DATA send-intent slices,
  request-side, response-side, request-trailer, and response-trailer
  header-list validation and final `204`/`304` no-content response lifecycle,
  the source-visible request `:path` value rule, the source-visible request
  `:scheme` HPACK static-name literal value rule, the source-visible `te`
  header value rule, response-side production `content-length`
  header-list consistency beyond the checked fixture boundary,
  accepted `content-length` body-length
  accounting for tracked inbound and outbound DATA,
  the malformed Huffman padding fixture diagnostic, and the outbound HPACK
  fixture header-list encoder slice, including exact static-indexed outbound
  helper bytes for finite HPACK static table fixed-value entries, static-name
  literal fixtures, visible-ASCII ordinary new-name literal-without-indexing for outbound
  HEADERS and `PUSH_PROMISE`, visible-ASCII ordinary new-name
  literal-never-indexed for outbound HEADERS without dynamic insertion,
  full-table single-byte Huffman-marked string literal decoding and encoding,
  source-visible payload-only HPACK Huffman encode boundary coverage, and the
  bounded
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
  diagnostics, general leading-update sequence state transitions, checked outbound
  dynamic table-size update encoding and state handoff into later HEADERS
  and server-side `PUSH_PROMISE`, checked outbound dynamic-table eviction
  after zero and reduced table-size updates, received peer
  `SETTINGS_HEADER_TABLE_SIZE` values driving later outbound HPACK fixture
  capacity, checked outbound dynamic-name literal-without-indexing,
  literal-with-indexing, and literal-never-indexed fixture encoding with raw
  and Huffman values, checked outbound new literal Huffman names across all
  three forms with raw and Huffman values, and checked outbound `PUSH_PROMISE`
  rejection after peer
  `SETTINGS_ENABLE_PUSH = 0`, checked outbound `PUSH_PROMISE`
  rejection above received or locally sent GOAWAY boundaries, plus
  deterministic `hpack-bytes-*` multi-byte non-visible Huffman fixture labels,
  the source-visible HPACK static-indexed decoder for every single-byte
  static table entry from `0x81` `:authority` through `0xbd`
  `www-authenticate:`, plus source-visible
  literal-without-indexing, literal-with-indexing, and
  literal-never-indexed static-name decoding of names resolved through the
  HPACK static table metadata with raw visible-ASCII values and bounded
  Huffman-marked literal values decoded through the HPACK static Huffman
  table, including line feed, single-byte `hpack-byte-*` labels, and
  multi-byte `hpack-bytes-*` labels, narrowed request `:scheme` static-name
  paths for accepted `http` and `https` values and rejected visible ASCII
  values, narrowed request `:path: test` Huffman static-name receive through
  completed HEADERS and final CONTINUATION before fixture fallback,
  narrowed request `:authority` static-name paths for accepted and rejected
  visible ASCII values, plus narrowed request and response
  `content-length` static-name paths
  for accepted visible ASCII decimal values on literal-without-indexing,
  literal-with-indexing, and literal-never-indexed forms that do not observe
  later fixture dynamic-table reuse, and the source-visible dynamic indexed
  HPACK core boundary for multiple carried bounded dynamic-table entries and
  saturated seven-bit indexed representation `0xff 0x00`, plus the
  source-visible HPACK integer core for checked indexed-field, table-size
  update, and string-literal-length prefix widths, including bounded encoding
  and focused non-terminating continuation diagnostics, plus the
  source-visible HPACK dynamic-table accounting core for the entry-size
  formula, newest-first insertion, table-size reduction, insertion-caused
  eviction, over-limit insertion, and zero-size table reduction, plus the
  source-visible static-name literal-with-indexing receive core for checked
  dynamic-state insertion and `0xbe` dynamic-indexed reuse, plus the
  source-visible raw literal-name Huffman-value receive core for checked
  Huffman-marked values, dynamic-state insertion, and `0xbe` dynamic-indexed
  reuse, plus the source-visible dynamic-name literal receive core for checked
  literal-without-indexing, literal-with-indexing, and literal-never-indexed
  forms whose names come from the carried bounded dynamic table, plus the
  source-visible dynamic-name Huffman-value receive core for checked
  Huffman-marked values and bounded dynamic-state insertion, plus the
  source-visible exact static-indexed outbound encode helper for the finite
  HPACK static table entries with fixed values across outbound HEADERS and
  server-side `PUSH_PROMISE`, plus source-visible static-name
  literal-without-indexing outbound helper bytes for finite HPACK static
  table names with raw visible-ASCII values,
  recorded under `../specification/` and
  `../reference/implemented-proposals/`. Planned work still includes
  broader protocol-core behavior and remaining HPACK behavior beyond the
  legacy focused string-fixture boundaries and the implemented production
  inbound and outbound octet-value paths.
  The completed stream-identifier domain-value slice is archived under
  [HTTP/2 Stream Domain Values](../reference/implemented-proposals/http2-stream-domain-values.md).
  The completed source-visible static table decode slice is archived under
  [HTTP/2 HPACK Static Table Decode](../reference/implemented-proposals/http2-hpack-static-table-decode.md).
  The completed source-visible static-name Huffman literal slice is archived
  under
  [HTTP/2 HPACK Static-Name Huffman Literals](../reference/implemented-proposals/http2-hpack-static-name-huffman-literals.md).
  The completed source-visible HPACK Huffman decode boundary slice is archived
  under
  [HTTP/2 HPACK Huffman Decode Boundary](../reference/implemented-proposals/http2-hpack-huffman-decode-boundary.md).
  The completed source-visible HPACK Huffman encode boundary slice is archived
  under
  [HTTP/2 HPACK Huffman Encode Boundary](../reference/implemented-proposals/http2-hpack-huffman-encode-boundary.md).
  The completed source-visible dynamic indexed core slice is archived under
  [HTTP/2 HPACK Dynamic Index Core](../reference/implemented-proposals/http2-hpack-dynamic-index-core.md).
  The completed source-visible HPACK integer core slice is archived under
  [HTTP/2 HPACK Integer Core](../reference/implemented-proposals/http2-hpack-integer-core.md).
  The completed source-visible dynamic-table accounting core slice is archived
  under
  [HTTP/2 HPACK Dynamic Table Accounting Core](../reference/implemented-proposals/http2-hpack-dynamic-table-accounting-core.md).
  The completed source-visible dynamic-name literal receive core slice is
  archived under
  [HTTP/2 HPACK Dynamic-Name Literal Core](../reference/implemented-proposals/http2-hpack-dynamic-name-literal-core.md).
  The completed source-visible dynamic-name Huffman-value receive slice is
  archived under
  [HTTP/2 HPACK Dynamic-Name Huffman Values](../reference/implemented-proposals/http2-hpack-dynamic-name-huffman-values.md).
  The completed source-visible static-name indexing core slice is archived
  under
  [HTTP/2 HPACK Static-Name Indexing Core](../reference/implemented-proposals/http2-hpack-static-name-indexing-core.md).
  The completed `content-length` header-list validation and body-accounting
  slices, including source-visible static-name `content-length`, are archived
  under
  [HTTP/2 Content-Length Header Validation](../reference/implemented-proposals/http2-content-length-header-validation.md)
  and
  [HTTP/2 Content-Length Body Accounting](../reference/implemented-proposals/http2-content-length-body-accounting.md).
  The completed response `:status` value validation slice is archived under
  [HTTP/2 Response Header Validation](../reference/implemented-proposals/http2-response-header-validation.md).
  The completed ordinary `CONNECT` request-header validation slice is archived
  under
  [HTTP/2 CONNECT Request Header Validation](../reference/implemented-proposals/http2-connect-request-header-validation.md).
  The completed `SETTINGS_ENABLE_CONNECT_PROTOCOL` and extended CONNECT
  request-header slice is archived under
  [HTTP/2 Extended CONNECT Negotiation](../reference/implemented-proposals/http2-extended-connect-negotiation.md).
  The completed half-closed-by-peer outbound DATA send-intent slice is archived
  under
  [HTTP/2 Half-Closed-By-Peer Outbound DATA](../reference/implemented-proposals/http2-half-closed-by-peer-outbound-data.md).
  The completed half-closed-local PRIORITY receive slice is archived under
  [HTTP/2 Half-Closed-Local PRIORITY Receive](../reference/implemented-proposals/http2-half-closed-local-priority-receive.md).
  The completed outbound DATA send-credit refill from peer `WINDOW_UPDATE` and
  `SETTINGS_INITIAL_WINDOW_SIZE` delta slices are covered by
  [HTTP/2 Outbound DATA Flow Control](../reference/implemented-proposals/http2-outbound-data-flow-control.md).
  The completed list-backed multi-stream outbound DATA credit slice is
  archived under
  [HTTP/2 Multi-Stream Outbound Flow Control](../reference/implemented-proposals/http2-multi-stream-outbound-flow-control.md).
  The completed flow-control numeric domain-type slice is archived under
  [HTTP/2 Flow-Control Numeric Domain Types](../reference/implemented-proposals/http2-flow-control-numeric-domain-types.md).
  The completed outbound DATA post-GOAWAY send-intent boundary is archived
  under
  [HTTP/2 Outbound DATA GOAWAY Boundary](../reference/implemented-proposals/http2-outbound-data-goaway-boundary.md).
  The completed GOAWAY receive lifecycle slice is archived under
  [HTTP/2 GOAWAY Receive Lifecycle](../reference/implemented-proposals/http2-goaway-receive-lifecycle.md).
  The completed GOAWAY drain completion slice is archived under
  [HTTP/2 GOAWAY Drain Completion](../reference/implemented-proposals/http2-goaway-drain-completion.md).
  The completed accepted `content-length` body accounting slices are archived
  under
  [HTTP/2 Content-Length Body Accounting](../reference/implemented-proposals/http2-content-length-body-accounting.md).
  The completed dynamic-name continuation diagnostic slice is archived under
  [HTTP/2 HPACK Dynamic Name Continuation Diagnostics](../reference/implemented-proposals/http2-hpack-dynamic-name-continuation-diagnostics.md).
  The completed outbound dynamic-name literal-without-indexing fixture slice is
  archived under
  [HTTP/2 Outbound HPACK Dynamic-Name Literal](../reference/implemented-proposals/http2-outbound-hpack-dynamic-name-literal.md).
  The completed outbound dynamic-name literal-with-indexing fixture slice is
  archived under
  [HTTP/2 Outbound HPACK Dynamic-Name Indexed Literal](../reference/implemented-proposals/http2-outbound-hpack-dynamic-name-indexed-literal.md).
  The completed outbound dynamic-name literal-never-indexed fixture slice is
  archived under
  [HTTP/2 Outbound HPACK Dynamic-Name Never-Indexed Literal](../reference/implemented-proposals/http2-outbound-hpack-dynamic-name-never-indexed-literal.md).
  The completed outbound dynamic-name Huffman-value fixture slice is archived
  under
  [HTTP/2 Outbound HPACK Dynamic-Name Huffman Values](../reference/implemented-proposals/http2-outbound-hpack-dynamic-name-huffman-values.md).
  The completed outbound Huffman literal-name fixture slice is archived under
  [HTTP/2 Outbound HPACK Huffman Literal Names](../reference/implemented-proposals/http2-outbound-hpack-huffman-literal-names.md).
  The completed bounded outbound representation-selection slice is archived
  under
  [HTTP/2 Outbound HPACK Representation Selection](../reference/implemented-proposals/http2-outbound-hpack-representation-selection.md).
  The completed production outbound ordered header-list encoder is archived
  under
  [HTTP/2 Production Outbound HPACK Header-List Encoding](../reference/implemented-proposals/http2-production-outbound-hpack-header-list-encoding.md).
  Automatic raw-or-Huffman selection for its literal strings is archived
  under
  [HTTP/2 Automatic Outbound HPACK Huffman Selection](../reference/implemented-proposals/http2-automatic-outbound-hpack-huffman-selection.md).
  The completed production inbound ordered header-list decoder is archived
  under
  [HTTP/2 Production Inbound HPACK Header-List Decoding](../reference/implemented-proposals/http2-production-inbound-hpack-header-list-decoding.md).
  Its completed production inbound octet-value follow-up is archived under
  [HTTP/2 Production Inbound HPACK Octet Values](../reference/implemented-proposals/http2-production-inbound-hpack-octet-values.md).
  Its completed production outbound octet-value follow-up is archived under
  [HTTP/2 Production Outbound HPACK Octet Values](../reference/implemented-proposals/http2-production-outbound-hpack-octet-values.md).
  The completed outbound ordinary literal-with-indexing fixture slice is
  archived under
  [HTTP/2 Outbound HPACK Ordinary Indexed Literal](../reference/implemented-proposals/http2-outbound-hpack-ordinary-indexed-literal.md).
  The completed outbound static-name literal-without-indexing fixture slice
  is archived under
  [HTTP/2 Outbound HPACK Static-Name Literal](../reference/implemented-proposals/http2-outbound-hpack-static-name-literal.md).
  The completed outbound HPACK fixture encoder slice is archived under
  [HTTP/2 Outbound HPACK Fixture Encoder](../reference/implemented-proposals/http2-outbound-hpack-fixture-encoder.md).
  The completed outbound HPACK dynamic-table eviction slice is archived under
  [HTTP/2 Outbound HPACK Dynamic Table Eviction](../reference/implemented-proposals/http2-outbound-hpack-dynamic-table-eviction.md).
  The completed outbound `PUSH_PROMISE` peer enable-push setting slice is
  archived under
  [HTTP/2 Outbound PUSH_PROMISE Enable-Push Setting](../reference/implemented-proposals/http2-outbound-push-promise-enable-push-setting.md).
  The completed outbound `PUSH_PROMISE` post-GOAWAY send-intent boundary is
  archived under
  [HTTP/2 Outbound PUSH_PROMISE GOAWAY Boundary](../reference/implemented-proposals/http2-outbound-push-promise-goaway-boundary.md).
  The completed outbound `PRIORITY` post-GOAWAY send-intent boundary is
  archived under
  [HTTP/2 Outbound PRIORITY GOAWAY Boundary](../reference/implemented-proposals/http2-outbound-priority-goaway-boundary.md).
  The completed outbound SETTINGS ACK send-intent slice is archived under
  [HTTP/2 SETTINGS ACK Send State](../reference/implemented-proposals/http2-settings-ack-send-state.md).
  The completed SETTINGS item-length validation slice is archived under
  [HTTP/2 SETTINGS Item-Length Validation](../reference/implemented-proposals/http2-settings-item-length-validation.md).
  The completed duplicate known SETTINGS receive-ordering and atomic-rejection
  slice is archived under
  [HTTP/2 Duplicate SETTINGS Items](../reference/implemented-proposals/http2-duplicate-settings-items.md).
  The completed ordered local SETTINGS batch send-intent slice, including
  local four-byte SETTINGS value-field representability checks, is archived
  under
  [HTTP/2 Local SETTINGS Batch Send](../reference/implemented-proposals/http2-local-settings-batch-send.md).
  The completed inbound dynamic-table fixture slice is archived under
  [HTTP/2 HPACK Dynamic Table Fixture](../reference/implemented-proposals/http2-hpack-dynamic-table-eviction-fixture.md).
- [Network Effect Integration Boundary](network-effect-integration-boundary.md):
  define remaining transport adapter, richer production socket APIs, richer
  stream-routing, channel, and task behavior
  beyond the implemented transport, bounded channel-first routing, general
  receiver-list routing, task, relative and absolute monotonic deadline,
  cancellation, deadline-aware listener
  accept, cancellable deadline-aware listener accept, deadline-aware stream
  read, cancellable deadline-aware stream read, cancellable deadline-aware
  stream write, accepted-stream endpoint text inspection,
  adapter-owned source-visible client connect,
  listener-to-clean-stream-end lifecycle,
  context-based adapter
  `task::spawn_with<Result, Context>` helper routing,
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
  HTTP/2 adapter/core write projection through ordered `net::write_chunks`,
  production-loopback listen, sequential accept, read, write, clean listener
  end, close lifecycle, two-stream adapter handler/action lifecycle, and
  listener-drain adapter lifecycle, listener-drain read-failure runtime
  boundary, source-visible adapter accept-loop helper, deadline-aware adapter
  lifecycle, deadline-aware accept and read failure runtime boundaries,
  production cancellable deadline-aware adapter lifecycle and outcome
  boundary, adapter close-failure runtime boundary, and explicit
  listener-close boundary, adapter-owned cancellation owner
  lifecycle boundary, production owner-drain cancellable deadline lifecycle
  boundary, production multi-chunk adapter event routing, production
  multi-event adapter task-helper routing, production multi-chunk
  read-failure runtime boundary, per-stream task handler-failure lifecycle
  boundary, concurrent stream task drain
  ([Network Concurrent Stream Task Drain](../reference/implemented-proposals/network-concurrent-stream-task-drain.md)),
  fail-fast cancellation and reclamation of pending stream tasks after drain
  failure
  ([Network Cancel Pending Stream Tasks After Drain Failure](../reference/implemented-proposals/network-cancel-pending-stream-tasks-after-drain-failure.md)),
  source-visible standard stream adapter routing helper,
  accepted-stream endpoint text inspection, cancellation owner status query,
  absolute monotonic deadline
  construction
  ([Network Deadline At Boundary](../reference/implemented-proposals/network-deadline-at-boundary.md)),
  general
  receiver-list channel-first routing through the
  current checked select-many boundary including stale route-count fixture
  cleanup, receiver-list cancellable channel-first completion routing,
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
  The completed read-side stream half-close boundary is archived under
  [Network Stream Shutdown Read Boundary](../reference/implemented-proposals/network-stream-shutdown-read-boundary.md).
  The completed explicit listener-close boundary is archived under
  [Network Listener Close Boundary](../reference/implemented-proposals/network-listener-close-boundary.md).
  The completed source-visible client connect boundary is archived under
  [Network Client Connect Boundary](../reference/implemented-proposals/network-client-connect-boundary.md).
  The completed source-visible production listener/client pairing boundary is
  archived under
  [Network Production Listen Connect Lifecycle](../reference/implemented-proposals/network-production-listen-connect-lifecycle.md).
  The completed adapter-owned multi-handler outbound write-ordering and
  outbound write-failure slices are archived under
  [Network Adapter Outbound Write Ordering](../reference/implemented-proposals/network-adapter-outbound-write-ordering.md).
  The completed HTTP/2 adapter/core write boundary is archived under
  [Network HTTP/2 Adapter Core Write Boundary](../reference/implemented-proposals/network-http2-adapter-core-write-boundary.md).
  The completed adapter-owned clean shutdown slice is archived under
  [Network Adapter Clean Shutdown](../reference/implemented-proposals/network-adapter-clean-shutdown.md).
  The completed adapter-owned cancellation owner slice is archived under
  [Network Cancel Owner Boundary](../reference/implemented-proposals/network-cancel-owner-boundary.md).
  The completed cancellation owner status query slice is archived under
  [Network Cancel Owner Status](../reference/implemented-proposals/network-cancel-owner-status.md).
  The completed production owner-drain cancellable deadline lifecycle slice is
  archived under
  [Network Production Owner-Drain Lifecycle](../reference/implemented-proposals/network-production-owner-drain-lifecycle.md).
  The completed production multi-chunk event routing slice is archived under
  [Network Production Multi-Chunk Routing](../reference/implemented-proposals/network-production-multi-chunk-routing.md).
  The same record includes the completed production multi-event adapter
  task-helper routing evidence, production multi-chunk read-failure runtime
  boundary, and per-stream task handler-failure lifecycle cleanup boundary.
  The completed source-visible standard stream adapter routing helper slice
  is archived under
  [Network Stream Adapter Routing Helper](../reference/implemented-proposals/network-stream-adapter-routing-helper.md).
  The completed source-visible adapter accept-loop helper slice is archived
  under
  [Network Adapter Accept-Loop Helper](../reference/implemented-proposals/network-adapter-accept-loop-helper.md).
  The completed adapter-level cancellable write-drain helper slice is
  archived under
  [Network Adapter Cancellable Write-Drain](../reference/implemented-proposals/network-adapter-cancellable-write-drain.md).
  The completed production two-stream multi-cycle routing slice is archived
  under
  [Network Production Two-Stream Multi-Cycle Routing](../reference/implemented-proposals/network-production-two-stream-multi-cycle-routing.md).
  The completed source-visible stream state inspection slice is archived
  under
  [Network Stream State Inspection](../reference/implemented-proposals/network-stream-state-inspection.md).
  The completed accepted-stream endpoint text inspection slice is archived
  under
  [Network Stream Address Metadata](../reference/implemented-proposals/network-stream-address-metadata.md).
  The completed listener endpoint text inspection slice is archived under
  [Network Listener Address Metadata](../reference/implemented-proposals/network-listener-address-metadata.md).
  Deadline and cancellation behavior is complete for this proposal at the
  current relative and absolute monotonic `Deadline`, `CancelToken`,
  cancellation status-query, cancellable wait-outcome, cancellable
  deadline-aware listener accept, stream read, stream write, accepted-stream
  lifecycle, and cancellation owner/token/status boundary.
  The completed receiver-list select-many routing, cancellable completion
  routing, and stale route-count fixture cleanup slice is archived under
  [Network Channel Select-Many Routing](../reference/implemented-proposals/network-channel-select-many-routing.md).

## Update When

- New proposal work is added, split, superseded, completed, or removed.
- Proposal work becomes implemented and the resulting behavior is documented
  under `../specification/`.
- A completed proposal record moves to
  `../reference/implemented-proposals/`.
