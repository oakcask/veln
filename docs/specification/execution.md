# Execution Boundary

This page routes implemented execution facts. Open
[execution-full.md](execution-full.md) only when a short route here is not
enough.

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
- Standard byte chunk and byte view helpers execute as pure prelude runtime
  operations and return immutable byte values or `Result` failures for invalid
  values, invalid compact hex fixture text, out-of-bounds counts and ranges,
  fixed-width unsigned read truncation, schema fixed-field mismatches, bounded
  view slicing, and conversion overflow.
- `ByteView`, `ByteChunk`, `StreamInput`, `DecodeStep<T>`,
  `DecodeReadiness`, `DecodeError`, `EncodeStep<TState>`, and `EncodeError`
  values execute as ordinary immutable source-visible values.
- `net` and `time` calls are host runtime boundaries. Fixture-backed and
  production-loopback transport paths preserve the same source-visible result
  shapes while keeping socket, deadline, cancellation, and monotonic-clock
  work outside pure protocol code. Production-loopback cases can preserve more
  than one configured read chunk for one accepted stream; each chunk is
  observed by source as a separate read result before clean end.

## Binary Schemas

- Explicit schema decode expressions lower to the generated decode-step
  boundary for the referenced eligible binary schema. They use the supplied
  `ByteView` as bounded input and the supplied `ByteOffset` for consumed-count
  and diagnostic offset accounting, returning `DecodeStep<T>` for the
  schema-local visible record shape. Public schema aliases lower through the
  same generated boundary as the aliased schema.
- Explicit schema encode expressions lower to the generated encode boundary
  for the referenced eligible binary schema. They typecheck the supplied value
  against the schema-local visible record shape and return
  `Result<ByteChunk, EncodeError>`. Public schema aliases lower through the
  same generated boundary as the aliased schema.
- Compatibility generated binary schema decode helpers read fields in
  declaration order and return the schema-local visible record shape. They are
  retained for old fixtures and runtime adapter coverage, not as the public
  source surface for applying schemas.
- Format-neutral generated decode helpers for schemas without a `format`
  clause accept a schema-local visible record shape and return
  `Result<T, String>`. The helper returns the supplied record on success and
  is limited to scalar fields, top-level `List<Int>`, `List<Bool>`,
  `List<Float>`, or `List<String>` fields, top-level `Dict<String, Int>`,
  `Dict<String, Bool>`, `Dict<String, Float>`, or `Dict<String, String>`
  fields, nested record-shaped fields that use scalar, `List<scalar>`, or
  `Option<scalar>` field types, and `Option<T>` fields whose payload is one
  of those scalar or nested record shapes.
- Repeated fields written as `[Payload; count]` normalize to the same generated
  decode and encode helper behavior as `Repeat(count, Payload)`, with the
  payload before `;` and the count expression after it. The count expression
  uses the same earlier-field and arithmetic forms accepted by `Repeat`.
- Dispatch payload cases written with lowercase exact-width `uint...` and
  `flag...` primitive spelling normalize to the same generated decode and
  encode helper behavior as compatible upper-case exact-width payload spelling.
  Byte-aligned lowercase `uint... reserves <value>` dispatch payloads validate
  the fixed payload bytes during decode, emit those bytes during encode, and
  expose `()` as the payload value.
- Same-module recursive dispatch payload cases expose a finite primitive
  payload shape when the recursive dispatch field is length-bounded and has a
  non-recursive primitive base case. Decode helpers collapse recursive known
  payload chains to that primitive payload value. Encode helpers accept the
  same schema-local visible shape and can encode the primitive base case while
  preserving the usual dispatch length checks. The checked success case is
  `examples/specification/run/binary-schema-recursive-dispatch-decode-encode/`;
  the checked missing-base rejection case is
  `examples/specification/run/binary-schema-recursive-dispatch-rejected/`.
- Representation-only fields such as supported `ReservedBits(width, value)`
  and lowercase `uint... reserves <value>` layouts are validated and omitted
  from the decoded record.
- Generated `validate_<schema>` helpers accept the schema-local decoded record
  shape and check field-local `where` predicates plus the single schema-level
  `validate` predicate when present.
- Generated binary schema encode helpers accept the schema-local visible
  record shape, validate field-local and representation constraints, and write
  bytes through the declared schema layout. Lowercase reserved-bit fields emit
  their declared values and are omitted from the input record like compatible
  `ReservedBits(width, value)` fields.
- Representation-local generated schema encode failures that cannot write a
  supplied value, repeat count, or length-bounded `ByteView` use
  `schema.encode_value_unrepresentable` while preserving the existing
  `EncodeError` field path and reason shape. Hand-written codec
  `EncodeError(...)` values may still use codec-owned ids.
- Representation-local generated schema dispatch encode failures use
  `schema.dispatch_unknown_tag`, `schema.dispatch_length_mismatch`, and
  `schema.dispatch_mismatch` for unknown closed-dispatch tags, extension
  dispatch payload length mismatches, and extension dispatch tag/payload
  mismatches. Compatibility-only hand-written `EncodeError(...)` values may
  still use the corresponding `codec.dispatch_*` ids.
- Projection between a schema-local record and a domain value is ordinary Veln
  source at the caller or schema-operation boundary. The checked schema-local projection
  case is
  `examples/specification/run/binary-schema-local-projection-boundary/`.
- Schema-level `map to` clauses are rejected by the parser before execution.
  The checked rejection case is
  `examples/specification/check/schema-map-to-rejected/`.

## Codecs

- Source-level `codec` and `pub codec` declarations are rejected before
  execution. Executable decode and encode entry points are ordinary functions
  or explicit schema operation expressions.
- Compatibility-only runtime diagnostic ids under `codec.*` remain part of
  runtime diagnostic vocabulary where existing runtime values use them.
- The source-visible HPACK static decoder accepts static indexed fields,
  bounded static-name literal-without-indexing fields, bounded static-name
  literal-with-indexing fields, and bounded static-name literal-never-indexed
  fields for names resolved through the HPACK static table metadata, including
  raw visible-ASCII values and bounded Huffman-marked literal values decoded
  through the HPACK static Huffman table. The checked static boundary includes
  visible ASCII, line feed, single-byte `hpack-byte-*` labels, and multi-byte
  `hpack-bytes-*` labels across the static-name literal forms under
  `examples/specification/run/hpack-static-codec-boundary/`. Stateful
  HTTP/2 request decoding also accepts `content-length` through the static-name
  literal forms checked by `examples/specification/run/http2-protocol-core/`
  when no later fixture dynamic-table reuse is observed. Stateful HTTP/2
  header-block decoding still routes literal-with-indexing blocks through the
  HPACK fixture decoder when fixture dynamic-table state must be updated.
- The source-visible HPACK fixture encoder accepts the checked outbound
  dynamic-name literal-with-indexing slice under
  `examples/specification/run/hpack-fixture-codec-boundary/` and routes the
  returned encode state through outbound HEADERS and server-side
  `PUSH_PROMISE` framing in
  `examples/specification/run/http2-protocol-core/`.
- The checked HTTP/2 protocol core records one pending empty SETTINGS ACK
  send intent after a valid non-ACK peer SETTINGS frame with payload items.
  Multiple peer SETTINGS frames received before consumption coalesce to that
  one pending ACK. Consuming the intent emits an empty SETTINGS frame with the
  ACK flag and clears the pending ACK state without mutating peer-advertised
  settings. The checked case is
  `examples/specification/run/http2-protocol-core/`.
- The same checked HTTP/2 protocol core emits local SETTINGS send-intents as
  one frame-header-plus-payload chunk whose payload preserves caller item
  order. Supported local batch items are
  `SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_ENABLE_PUSH`,
  `SETTINGS_INITIAL_WINDOW_SIZE`, `SETTINGS_MAX_CONCURRENT_STREAMS`,
  `SETTINGS_MAX_FRAME_SIZE`, and `SETTINGS_MAX_HEADER_LIST_SIZE`. Accepted
  batches are recorded as one outstanding local batch; one valid peer
  SETTINGS ACK clears exactly the oldest outstanding batch and leaves later
  batches pending. The checked case also fixes the no-output
  `local_settings` range-diagnostic path for an invalid item inside a larger
  batch.

## Runtime Output

- `veln run` entries project returned `Result`, `ByteChunk`,
  `List<ByteChunk>`, HTTP/2 protocol diagnostics, and runtime diagnostic
  values through the command output boundary described in
  [commands.md](commands.md), [json-output.md](json-output.md), and
  [test-json.md](test-json.md).
- Executable specification cases may declare named binary fixture records and
  named output chunks in `case.toml`. These fixture records are harness
  expectations, not language syntax.
- Tail-recursive user functions may execute deep self-recursive calls through
  the implemented trampoline path. Other JVM details are backend details unless
  this reference marks a behavior as observable.

## Read When

- Core, typed IR, selected-entry reachability, and stdio ordering:
  [commands.md](commands.md).
- JVM lowering support, runtime containers, file-system, network, time, and
  binary helper details: [execution-full.md](execution-full.md).
- Machine-readable command output: [json-output.md](json-output.md) and
  [test-json.md](test-json.md).

## Skip Unless Needed

- Use [commands.md](commands.md) first for command gates and user-facing
  command behavior.
- Use checked examples before expanding this routing page.
