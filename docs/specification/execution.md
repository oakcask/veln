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
  work outside pure protocol code.

## Binary Schemas

- Explicit schema decode expressions lower to the generated decode-step
  boundary for the referenced eligible binary schema. They use the supplied
  `ByteView` as bounded input and the supplied `ByteOffset` for consumed-count
  and diagnostic offset accounting, returning `DecodeStep<T>` for the
  schema-local visible record shape.
- Explicit schema encode expressions lower to the generated encode boundary
  for the referenced eligible binary schema. They typecheck the supplied value
  against the schema-local visible record shape and return
  `Result<ByteChunk, EncodeError>`.
- Compatibility generated binary schema decode helpers read fields in
  declaration order and return the schema-local visible record shape. They are
  retained for old fixtures and runtime adapter coverage, not as the public
  source surface for applying schemas.
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
