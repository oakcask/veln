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
- [HTTP/2 Sans-I/O Protocol Core](http2-sans-io-protocol-core.md): select
  only remaining SETTINGS, DATA, stream-lifecycle, graceful-shutdown, typed
  protocol-error, or HPACK work that is absent from the executable HTTP/2
  specification. Check current specification and implemented-proposal routes
  before choosing a slice; completed behavior is not inventoried here.
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
