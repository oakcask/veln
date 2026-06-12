# Execution Boundary

This page routes implemented execution facts. Use it before opening the full
execution reference.

## Read First

- Checked core and typed IR are produced only after semantic diagnostics have
  no errors.
- Shared command analysis keeps checked-core readiness and selected-entry
  typed-IR readiness before command-specific execution or write policy.
- Reachable executable blockers include holes, missing expressions,
  constructor arity gaps, call arity gaps, and recognized concurrency calls.
- The ordinary JVM execution path emits classfile artifacts directly; Java
  source generation and Java source compilation are not part of the observable
  command boundary.
- The generated JVM class cache validates manifests and classfile contents;
  invalid or incomplete entries are regenerated before execution.
- Standard `List` traversal helpers execute through runtime support that avoids
  growing the host call stack for large helper traversals.
- Standard byte chunk and byte view helpers execute as pure prelude runtime
  operations and return immutable byte values or `Result` failures for invalid
  values, invalid compact hex fixture text, out-of-bounds counts and ranges,
  truncation, schema fixed-field mismatches, and fixed-width unsigned
  conversion overflow. Standard `StreamInput`, `DecodeStep<T>`,
  `DecodeReadiness`, `DecodeError`, `EncodeStep<TState>`, and `EncodeError`
  values execute as ordinary immutable ADT values.
- The implemented binary schema primitive execution slice decodes the
  `Http2FrameHeader` field sequence from a `ByteView`: `UInt24be`, `UInt8`,
  `UInt8`, `ReservedBits(1, 0)`, and `UInt31be`. The decoded value exposes
  ordinary `Int` fields for `length`, `kind`, `flags`, and `stream_id`.
  The reserved field is consumed and validated but is not exposed in the
  mapped record. Truncated schema fields report `schema.truncated_field`;
  invalid reserved bits report `schema.reserved_bits_mismatch`. Both carry
  byte offset and schema field path details.
- The `SchemaWidthSample` primitive decode slice consumes `UInt16be` followed
  by `UInt32be` from a `ByteView`. Both visible fields decode to ordinary
  `Int` values. Truncated fields use the same `schema.truncated_field` byte
  diagnostic shape as the frame-header slice, including byte offset, field
  path, expected count, available count, readiness, and structured byte
  preview fields.
- The binary schema field-local validation slice decodes fields in declaration
  order for generated `byte_decode_<schema>` helpers when every field uses an
  implemented exact-width unsigned binary primitive. It checks each supported
  `where` predicate after its owning field is decoded. Predicates may use the
  current field and earlier decoded fields with comparison, boolean, literal,
  arithmetic, prefix `not`, and grouping forms. Later-field references,
  unknown fields, and ordinary source bindings named by a predicate are
  rejected as unsupported schema predicate references. A failed predicate
  reports `schema.validation_failed` at the owning field byte offset with
  schema field path, predicate text, decoded values, and structured byte
  preview fields.
- The narrow binary schema closed dispatch slice decodes
  `Dispatch(tag_field, tag => Payload, ...)` fields after the referenced tag
  field has been decoded by an earlier exact-width field in the same schema.
  Known dispatch cases consume either the selected exact-width unsigned
  payload primitive and expose an ordinary `Int` field, or the selected
  same-module nested binary schema and expose that schema's decoded record
  shape. Unknown tags in the closed dispatch report
  `schema.dispatch_unknown_tag` at the dispatch field byte offset with schema
  field path, decoded tag field, decoded tag value, expected tags, and
  structured byte preview fields. The checked examples are
  `examples/specification/run/binary-schema-closed-dispatch-decode/`,
  `examples/specification/run/binary-schema-closed-dispatch-nested-decode/`,
  `examples/specification/run/binary-schema-closed-dispatch-unknown-json/`,
  and
  `examples/specification/run/binary-schema-closed-dispatch-unknown-human/`.
- The narrow binary schema extension dispatch slice decodes
  `ExtensionDispatch(tag_field, length_field, tag => Payload, ...)` fields
  after both referenced fields have been decoded by earlier exact-width fields
  in the same schema. Known cases consume either the selected exact-width
  unsigned payload primitive or the selected same-module nested binary schema
  from the bounded payload bytes selected by `length_field`, then expose it as
  `SchemaDispatchPayload::Known(value)`. Unknown cases do not report
  `schema.dispatch_unknown_tag`; they expose
  `SchemaDispatchPayload::Unknown(tag, payload)` where `payload` is a bounded
  `ByteView` over exactly the byte count decoded from `length_field`. If the
  closed input cannot provide that bounded payload, the helper reports
  `schema.length_out_of_bounds` at the first missing payload byte. Nested
  payload decode failures report the nested schema field path and absolute byte
  offset from the enclosing input. The checked examples are
  `examples/specification/run/binary-schema-extension-dispatch-decode/`,
  `examples/specification/run/binary-schema-extension-dispatch-nested-decode/`,
  `examples/specification/run/binary-schema-extension-dispatch-unknown/`,
  `examples/specification/run/binary-schema-extension-dispatch-nested-unknown/`,
  `examples/specification/run/binary-schema-dispatch-nested-failure-json/`, and
  `examples/specification/run/binary-schema-extension-dispatch-length-human/`.
- When an eligible generated binary schema decode helper has one structural
  `map to Target` clause and the target resolves to a single record-shaped
  source type whose mapped fields match the schema-local decoded field types,
  the helper returns the mapped ordinary record shape instead of the
  schema-local field shape. Mapping assignment sources must name decoded
  schema fields. Mapping assignment targets must name target fields, and every
  target field must be assigned once before execution.
- Eligible generated binary schema decode-step helpers named
  `byte_decode_step_<schema>` accept a bounded `ByteView` and explicit base
  `ByteOffset`. When the view has at least the schema's exact-width byte
  count, they return `Decoded(value, consumed)` with `consumed` equal to the
  exact schema byte count. When the open view is shorter, they return
  `NeedMore(NeedBytes(count))` with `count` equal to the minimum buffered byte
  count required before retrying and consume no bytes. Closed-input
  `byte_decode_<schema>` truncation diagnostics remain on the existing
  `Result` helper path.
- Eligible generated binary schema encode helpers named
  `byte_encode_<schema>` accept one record whose fields match the schema-local
  visible exact-width unsigned primitive fields as ordinary `Int` values. A
  `ReservedBits(1, 0)` field immediately before a `UInt31be` field is
  representation-only: it is omitted from the record and the helper emits the
  required zero high bit in the shared four-byte stream identifier position.
  The helper writes fields in declaration order into one immutable big-endian
  `ByteChunk` and returns `Result<ByteChunk, EncodeError>`. Values outside the
  primitive range return `Err(EncodeError("codec.out_of_range", field_path,
  reason))`; `UInt31be` uses the 31-bit maximum even though it occupies four
  bytes. Unsupported reserved-bit encode shapes report
  `schema.reserved_bits_encode`. This slice excludes schema mappings,
  field-local validation, dispatch fields, other reserved or fixed fields,
  nested mappings, and derived codec encode execution for unsupported schemas.
  The checked examples are
  `examples/specification/run/binary-schema-primitive-encode/`,
  `examples/specification/run/binary-schema-primitive-encode-out-of-range/`,
  `examples/specification/run/binary-schema-reserved-bit-encode/`, and
  `examples/specification/check/schema-reserved-bit-encode-diagnostics/`.
- A codec declaration with a valid `derive encode` clause for the same
  eligible generated binary schema encode helper slice exposes the codec item
  name as the executable encode boundary for ordinary source calls. The call
  accepts the generated helper's value record, invokes the schema encode
  helper, returns `EncodeStep<()>`, projects helper `Ok(ByteChunk)` output to
  `Encoded(List<ByteChunk>)` with one chunk, and projects helper
  `Err(EncodeError)` output to `Invalid(EncodeError)`. The checked example is
  `examples/specification/run/derived-codec-encode-boundary/`.
- A codec declaration with a valid `derive decode` clause for the same
  eligible generated binary schema decode-step slice exposes the codec item
  name as the executable decode boundary for ordinary source calls. The call
  accepts a bounded `ByteView` and explicit base `ByteOffset` and returns the
  same `DecodeStep<T>` value as `byte_decode_step_<schema>`, including mapped
  record values, `NeedMore(NeedBytes(count))`, and `Invalid` without consumed
  bytes.
- A codec declaration with a valid hand-written `decode with function_name`
  clause exposes the codec item name as the executable decode boundary for
  ordinary source calls. The call accepts a bounded `ByteView` and explicit
  base `ByteOffset`, invokes the referenced same-module function, and returns
  its `DecodeStep<T>` unchanged.
- A codec declaration with a valid hand-written `encode with function_name`
  clause exposes the codec item name as the executable encode boundary for
  ordinary source calls. The call invokes the referenced same-module function
  with that function's parameters and returns its `EncodeStep<TState>` value
  unchanged. For the implemented single structural `map to Target` schema
  slice, the first encoder parameter remains the mapped target record shape.
- Same-module private decode codecs are callable only in their declaring
  module; same-module private encode codecs follow the same rule. Imported
  calls require a written qualified module path to a `pub codec`.
- The frame decode helper reuses the frame-header validation and adds a
  bounded `payload: ByteView` over the same bytes. The payload starts after
  the nine-byte frame header and uses the decoded `length` as its count. If
  the closed input cannot provide that payload range, the helper returns
  `schema.length_out_of_bounds` with byte offset, schema field path, expected
  payload count, available payload count, and structured byte preview fields.
- Executable specification cases may keep named binary fixture records in the
  example tree; the harness checks complete lowercase hex output without
  promoting a production fixture API. Named fixture records can also represent
  valid decoded bytes that are intentionally too short for a closed-input
  `ByteView` read; those cases keep fixture-owned truncation facts in
  metadata while `run --json` reports `codec.incomplete_input`. Other named
  fixture records can represent valid decoded bytes that fail a test-owned
  codec or protocol field check; their metadata records the diagnostic id,
  byte offset, structured field path, and consumed count where the case has
  one.
- Executable specification cases may also assert named output `ByteChunk`
  lists through complete lowercase hex in `case.toml`. The harness checks
  stable consecutive program-output lines for the list count, chunk order,
  exact hex strings, decoded byte counts, empty lists, and zero-length chunks.
- The first ordinary-source HTTP/2 sans-I/O protocol-core example models
  chunk arrival and end-of-stream events as ADTs. Its pure decode state keeps
  undecoded suffix bytes, the next absolute byte offset, continuation state,
  the active local receive-limit entry, peer-advertised SETTINGS state, and
  graceful shutdown state. It reuses the frame-header primitive for available
  headers and represents closed-input truncation, continuation ordering
  failures, incoming frame payloads that exceed the active receive maximum
  frame size, received `SETTINGS_MAX_FRAME_SIZE` values outside the accepted
  SETTINGS range, invalid connection-state and stream-state frame kinds,
  wrong-length PING and GOAWAY payloads, accepted PING ACK distinction, and
  accepted GOAWAY last-stream-id and error-code facts as typed protocol
  values. The receive flow-control state consumes DATA payload length from
  connection and stream windows, accepts connection-level and open-stream
  `WINDOW_UPDATE` increments, and keeps wrong-length, idle-stream, zero, and
  overflow `WINDOW_UPDATE` cases as typed protocol failures before projecting
  stable diagnostic ids and related context into fixture output, human runtime
  diagnostics, and `run --json`
  `protocol_diagnostic` details.
- Eligible direct tail-recursive user functions execute deep self-recursive
  chains without growing the host call stack for each logical step.
- Other JVM details are backend details unless this reference marks a behavior
  as an observable language boundary.

## Read When

- Core, typed IR, selected-entry reachability, and stdio ordering:
  [execution-full.md](execution-full.md#core-and-ir).
- JVM lowering support, runtime containers, file-system and process
  intrinsics, channels, tasks, contract failures, and the class cache:
  [execution-full.md](execution-full.md#jvm-backend).

## Skip Unless Needed

- Use [commands.md](commands.md) first for command gates and user-facing
  behavior.
- Use [json-output.md](json-output.md) first for machine-readable command
  output.
