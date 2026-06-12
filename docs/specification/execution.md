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
  `Dispatch(tag_field, tag => Primitive, ...)` fields after the referenced tag
  field has been decoded by an earlier exact-width field in the same schema.
  Known dispatch cases consume the selected exact-width unsigned payload
  primitive and expose the payload as an ordinary `Int` field. Unknown tags in
  the closed dispatch report `schema.dispatch_unknown_tag` at the dispatch
  field byte offset with schema field path, decoded tag field, decoded tag
  value, expected tags, and structured byte preview fields. The checked
  examples are
  `examples/specification/run/binary-schema-closed-dispatch-decode/`,
  `examples/specification/run/binary-schema-closed-dispatch-unknown-json/`,
  and
  `examples/specification/run/binary-schema-closed-dispatch-unknown-human/`.
- When an eligible generated binary schema decode helper has one structural
  `map to Target` clause and the target resolves to a single record-shaped
  source type whose mapped fields are `Int`, the helper returns the mapped
  ordinary record shape instead of the schema-local field shape. Mapping
  assignment sources must name decoded schema fields. Mapping assignment
  targets must name target fields, every target field must be assigned once,
  and non-`Int` target fields are rejected before execution.
- Eligible generated binary schema decode-step helpers named
  `byte_decode_step_<schema>` accept a bounded `ByteView` and explicit base
  `ByteOffset`. When the view has at least the schema's exact-width byte
  count, they return `Decoded(value, consumed)` with `consumed` equal to the
  exact schema byte count. When the open view is shorter, they return
  `NeedMore(NeedBytes(count))` with `count` equal to the minimum buffered byte
  count required before retrying and consume no bytes. Closed-input
  `byte_decode_<schema>` truncation diagnostics remain on the existing
  `Result` helper path.
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
  the active local receive-limit entry, and peer-advertised SETTINGS state. It
  reuses the frame-header primitive for available headers and represents
  closed-input truncation, continuation ordering failures, and incoming frame
  payloads that exceed the active receive maximum frame size, plus received
  `SETTINGS_MAX_FRAME_SIZE` values outside the accepted SETTINGS range and
  invalid connection-state and stream-state frame kinds, as typed protocol
  values before projecting stable diagnostic ids and related context into
  fixture output, human runtime diagnostics, and
  `run --json` `protocol_diagnostic` details.
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
