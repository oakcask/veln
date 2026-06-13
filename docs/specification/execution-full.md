# Execution Boundary

This file specifies the implemented execution boundary.

## Core And IR

Checked core is produced only after semantic diagnostics have no errors. Typed
IR is produced only when checked core is complete. Reachable holes, missing
expressions, constructor arity gaps, call arity gaps, and recognized
concurrency calls block executable IR. For selected `run` and `test` entries,
reachability includes direct function calls, bare and `use` alias-qualified
function declaration values used inside reachable expressions, and function
calls in reachable contract predicates. Reachability also follows bare and
`use` alias-qualified function declaration values passed as contract call
arguments. Calls through a function-typed local binding or parameter are
conservative: when the surface graph does not identify one concrete function
declaration, reachability includes visible function declarations with the same
argument count. In a named source module, a bare function reference resolves
reachability only to functions owned by that same source module. Qualified
calls and function values resolved through selected-file `use` aliases keep
the imported module identity, so same-named functions from other modules are
not included only because their local name matches. Bare local bindings,
parameters, and match-pattern bindings shadow same-named function declarations
for selected-entry reachability; a shadowed bare name is treated as the local
value, not as a function declaration value. The implemented execution fixtures
cover function declarations used as function-typed values, function-typed value
calls, opaque function-typed value call reachability, contract helper
reachability, contract function value reachability, imported-call
reachable-hole blocking, selected-entry reachable-hole blocking, local
shadowing of function declarations, and selected-entry concurrency blockers
before JVM execution.
When a function or test body omits the final expression line, checked core and
typed IR materialize that omission as an explicit `()` return.

The typed IR is runtime-neutral. JVM class names, Java method names, boxed
runtime representation, generated artifact paths, cache keys, and runtime
helper layout are backend details and are not language facts.

Stdio operations are serialized at the runtime handler boundary. Each
`stdio::print`, `stdio::println`, `stdio::eprint`, and `stdio::eprintln`
operation writes its complete logical output and records its test event while
holding the same handler lock. Captured event `sequence` values therefore
define one total operation order across stdout and stderr for a selected run or
test case, including calls made by spawned tasks.

## JVM Backend

The JVM backend emits classfile artifacts directly for the implemented IR
subset:

- functions, parameters, locals, expression statements, and returns
- omitted tail expressions as `()` returns
- literals, records, vecs, `Ok`, `Err`, `Some`, `None`, their `Result::` or
  `Option::` qualified forms, and `?`
- `match` expressions over literals, `_`, bindings, and built-in `Option` and
  `Result` constructors, after finite-domain exhaustiveness diagnostics have
  passed
- record field access
- stdio builtins, prelude helpers, ordinary function calls, and function-value
  calls
- file-system, network, time, and current-process standard library intrinsics
- bounded channel construction, sender clone, send, receive, and close calls
- two-receiver channel selection calls with optional timeout
- task spawn, join, and cancellation calls
- pipelines with named or qualified call targets lowered to calls with the
  left expression inserted as the first argument
- runtime `require` checks at function entry and runtime `ensure` checks before
  tail-expression returns and `?` early returns
- integer and boolean operators used by the implemented type rules

Generated runtime helpers may use mutable builders while constructing records,
vecs, and dictionary update results. Values returned to Veln user code are
frozen at that boundary: records and dictionaries are exposed as unmodifiable
maps, vecs are exposed as unmodifiable host lists, and prelude container updates
return new frozen containers instead of mutating the input value in place.
Standard `List` helper traversals, including `list_fold`, `list_reverse`,
`list_map`, `list_filter`, and `list_try_map`, execute through runtime support
that iterates over the list representation instead of growing the host call
stack. This support does not expose source-level tail-call syntax or a general
tail-call optimization guarantee.

Standard byte chunk and byte view helpers are pure prelude runtime operations.
The runtime constructs immutable `ByteChunk` values, computes `ByteCount`,
appends chunks without mutating inputs, decodes compact ASCII hex fixture
text, constructs bounded `ByteView` values, materializes bounded views as
chunks, and reads or writes fixed-width unsigned big-endian integer
representations. These helpers return `Result` failures for invalid byte
values, invalid hex fixture text, negative counts or offsets, slice or drop
counts that exceed the chunk length, view ranges that exceed the chunk length,
truncated reads, and fixed-width unsigned conversion overflow. Hex fixture
decoding accepts ASCII hex byte pairs with ASCII whitespace between complete
bytes only; invalid characters and dangling nibbles return stable fixture hex
error ids with decoded byte offset and nibble position in the error text. When
such a failure propagates out of a `run --json` entry as an `Err(String)`, the
result failure details also include the fixture text span, decoded
`ByteOffset`, nibble position, and nearby fixture text context. Byte views
cross task and channel freeze boundaries as ordinary immutable ADT values. The
exact host representation of byte chunks, byte views, counts, offsets, and
bytes is backend-owned.

The binary schema primitive execution slice exposes a narrow frame-header
decode helper over `ByteView`. It consumes a `UInt24be` length field, two
`UInt8` fields, one `ReservedBits(1, 0)` field, and one `UInt31be` stream id
field. Exact-width unsigned fields produce ordinary `Int` values in the
decoded record. The reserved field is representation-only: it advances the
decode position and validates the fixed bit pattern but is omitted from the
record. Truncated schema fields return a `schema.truncated_field` result
failure with expected and available byte counts. Reserved-bit mismatches
return `schema.reserved_bits_mismatch` with bit width, expected value, actual
value, structured byte preview fields, byte offset, and schema field path.

The `SchemaWidthSample` primitive decode helper consumes one `UInt16be` field
followed by one `UInt32be` field from a `ByteView`. Both fields produce
ordinary `Int` values in the decoded record. Truncation reports the same
`schema.truncated_field` diagnostic shape as the frame-header helper,
including byte offset, structured field path, expected byte count, available
byte count, readiness, and structured byte preview fields.

The binary schema field-local validation execution slice decodes fields in
declaration order for generated `byte_decode_<schema>` helpers when every
field uses an implemented exact-width unsigned binary primitive. It checks a
supported `where` predicate after the owning field is decoded. Predicate
evaluation may read the current field and earlier decoded fields and supports
comparison, boolean, literal, arithmetic, prefix `not`, and grouping forms.
Later-field references, unknown fields, and ordinary source bindings named by
a predicate return an unsupported schema predicate reference error. Passing
validation returns ordinary `Int` values for decoded fields unless the schema
has one eligible structural `map to Target` clause. In that mapped slice, the
generated helper constructs the target record field names from decoded schema
fields after all field-local `where` predicates pass. Mapping diagnostics
reject unknown source fields, unknown target fields, duplicate or missing
target fields, and decoded source fields whose types do not match their target
fields before execution. Failed validation returns `schema.validation_failed`
at the owning field byte offset with structured field path, predicate text,
owning field value, decoded values, and structured byte preview fields.

The same eligible generated binary schema slice also exposes
`byte_decode_step_<schema>` helpers. A decode-step helper receives the bounded
`ByteView` to inspect and an explicit base `ByteOffset` for the first byte in
that view. If the view contains the full exact-width field sequence, the
helper returns `Decoded(value, consumed)` where `value` has the same schema or
mapped record shape as `byte_decode_<schema>` and `consumed` is exactly the
schema byte width. If the open view is shorter than that width, the helper
returns `NeedMore(NeedBytes(count))`, where `count` is the minimum buffered
byte count required before retrying, and it consumes no bytes. This
incremental helper does not change the closed-input `Result` helper path:
closed truncation still reports `schema.truncated_field` through
`byte_decode_<schema>`.

A codec declaration with a valid `derive decode` clause for the same eligible
generated binary schema decode-step slice exposes the codec item name as an
executable decode boundary in ordinary source calls. The call accepts the
bounded `ByteView` and explicit base `ByteOffset` and returns the same
`DecodeStep<T>` value as `byte_decode_step_<schema>`, including a mapped record
value when the schema has the implemented single structural `map to Target`
record mapping. `Decoded` reports the exact consumed byte count; `NeedMore`
and `Invalid` consume no bytes. For the implemented single structural mapping
slice, `T` is the mapping target record shape when each assignment source has
the same implemented decoded field type as the target field.

A codec declaration with a valid hand-written `decode with function_name`
clause also exposes the codec item name as an executable decode boundary in
ordinary source calls. The call accepts the bounded `ByteView` and explicit
base `ByteOffset`, invokes the already-checked same-module decode function, and
returns that function's `DecodeStep<T>` value unchanged. The implemented
mapped-record checker still requires `T` to match the referenced schema's
single structural mapping target shape. Same-module private decode codecs are
callable only inside their declaring module; imported calls require a written
qualified module path to a `pub codec`.

A codec declaration with a valid hand-written `encode with function_name`
clause exposes the codec item name as an executable encode boundary in
ordinary source calls. The call invokes the already-checked same-module encode
function with that function's parameters and returns its
`EncodeStep<TState>` value unchanged. For the implemented single structural
`map to Target` schema slice, the first encoder parameter remains the mapped
target record shape. Same-module private encode codecs are callable only
inside their declaring module; imported calls require a written qualified
module path to a `pub codec`.

A codec declaration with a valid `derive encode` clause for the same eligible
generated binary schema encode helper slice exposes the codec item name as an
executable encode boundary in ordinary source calls. The call accepts the
generated helper's value record, invokes the generated schema encode helper,
and returns `EncodeStep<()>`. Successful helper output is projected from
`Ok(ByteChunk)` to `Encoded(List<ByteChunk>)` with one immutable output chunk.
Helper `Err(EncodeError)` output is projected to `Invalid(EncodeError)`.
Same-module private derived encode codecs are callable only inside their
declaring module; imported calls require a written qualified module path to a
`pub codec`. General generated encode helper behavior outside the exact-width
primitive, supported reserved-bit, closed dispatch, extension dispatch, and
same-module nested dispatch payload slices remains unimplemented. When a
mapped schema would require the codec item to accept the mapping target rather
than the generated helper's schema-local value record, the `derive encode`
clause is rejected with `codec.encode_value_type`.

Eligible generated binary schema encode helpers named
`byte_encode_<schema>` accept one record whose fields match the schema-local
visible exact-width unsigned primitive fields as ordinary `Int` values. A
`ReservedBits(1, 0)` field immediately before a `UInt31be` field is
representation-only: it is omitted from the record and the helper emits the
required zero high bit in the shared four-byte stream identifier position.
Closed `Dispatch(tag_field, tag => Payload, ...)` fields are eligible when
`tag_field` names an earlier visible exact-width unsigned field and every case
payload is an implemented exact-width unsigned primitive payload or an earlier
same-module binary schema payload. The record contains the visible tag field
and one payload field; nested schema payload fields use the selected nested
schema decoded record shape. The helper chooses the case from the encoded tag
value, writes the selected payload in declaration order, and reports
`codec.dispatch_unknown_tag` when the tag value has no case.
Extension-tolerant
`ExtensionDispatch(tag_field, length_field, tag => Payload, ...)` fields are
eligible for the same exact-width unsigned primitive or same-module nested
binary schema payload cases when both the tag and length fields are earlier
visible exact-width unsigned fields. The payload record field is
`SchemaDispatchPayload<T>`, where `T` is the selected primitive `Int` or nested
schema decoded record shape. `Known(value)` writes the payload selected by the
visible tag field. `Unknown(tag, payload)` writes the bounded raw bytes from
the `ByteView` only when the visible tag value is not a known case and matches
the unknown payload tag. The supplied length field remains explicit: the
helper rejects values whose encoded payload byte count differs from the
earlier length field with `codec.dispatch_length_mismatch`. Visible tag and
payload variant disagreements report `codec.dispatch_mismatch`.
The helper writes fields in declaration order into one immutable big-endian
`ByteChunk` and returns `Result<ByteChunk, EncodeError>`. Values outside the
primitive range return `Err(EncodeError("codec.out_of_range", field_path,
reason))`; nested schema encode failures keep the nested schema field path.
`UInt31be` uses the 31-bit maximum even though it occupies four bytes.
Unsupported reserved-bit encode shapes report `schema.reserved_bits_encode`.
This slice excludes schema mappings, field-local validation, imported or
generalized dispatch payload schemas, other reserved or fixed fields, nested
mappings, and derived codec encode execution for unsupported schemas.

The frame decode helper extends that slice with a bounded payload view. It
first applies the same header validation, then returns the visible header
fields plus `payload: ByteView`. The payload view shares the input chunk,
starts immediately after the nine-byte frame header, and uses the decoded
length field as its count. If the closed input has fewer payload bytes than
the decoded length, the helper returns `schema.length_out_of_bounds` with the
first missing byte offset, `Http2FrameHeader.payload` field path, expected
payload count, available payload count, and structured byte preview fields.

Standard `StreamInput` values execute as ordinary immutable source ADT values:
`Chunk(bytes)` preserves the supplied `ByteChunk`, including an empty chunk,
and `End` is a separate nullary variant.

Executable specification cases may define named binary fixture records inside
their example source or helper files. These test-owned records can carry the
fixture name, decoded `ByteChunk`, optional consumed `ByteCount`, and expected
invalid-fixture error text. The CLI toolchain harness compares their observable
output with complete lowercase hex strings, byte diagnostic metadata, and
stable error text from `case.toml`. A named fixture can decode successfully and
still be intentionally too short for a closed-input `ByteView` read; in that
case, `run --json` reports `codec.incomplete_input` rather than a fixture text
validation failure. A named fixture can also decode successfully and fail a
test-owned codec or protocol field check; in that case, the harness metadata
records the diagnostic id, byte offset, structured field path, and consumed
count where the case has one. This fixture support is limited to executable
specification evidence and does not add a production binary serialization or
fixture API. The same harness support can assert named output `ByteChunk`
lists through complete lowercase hex chunks in `case.toml`, preserving chunk
order and distinguishing empty lists from zero-length chunks.

The source-backed `byte_expect_fixed_u8_be` helper reads one byte from a
`ByteView`, returns `Ok(Int)` when it matches the expected fixed value, and
returns `Err(String)` with `schema.fixed_field_mismatch` byte diagnostic
details when the byte is present but differs. Truncated input remains
`codec.incomplete_input`.

User-defined `fn` declarations are stack-safe for direct self-recursive chains
when every direct self call appears in tail position and the function has no
runtime `ensure` or `invariant` clauses. The final expression of a
function body is tail position. For a tail-position `match`, each arm result
expression is tail position, recursively through nested tail-position
matches. A direct self call in binary or prefix operands, call arguments,
aggregate literals, field access, `?`, `let` initializers, match scrutinees,
or non-final expression statements is not tail position. Calls through
function-typed values are not tail-recursive steps and keep ordinary call
lowering. Eligible tail-recursive steps evaluate the next call arguments
before rebinding parameters for the next logical invocation. Runtime `require`
checks still run at each logical function entry. Non-tail recursion, mutual
recursion, indirect recursion, and functions with runtime return checks,
including runtime `ensure` or `invariant` clauses, keep ordinary call lowering
and do not receive a stack-safety guarantee. The lowering strategy is
backend-owned and does not expose trampoline classes, continuation layout,
syntax, annotations, warnings, or machine-readable eligibility output as
language behavior.

Bounded channel values are backend-owned runtime handles. `channel::bounded`
and `channel::bounded<T>` return a record with `tx` and `rx` fields.
`channel::clone(tx)` returns another sender endpoint for the same channel.
Sending freezes the sent value before crossing the channel boundary. On a
positive-capacity channel, sending waits while the queue is full and then
returns `Ok(())` after the value is queued. Receiving blocks until a queued
value is available or the sender endpoint is closed. It returns `Some(value)`
for a received value and `None` after the channel is closed and drained. A
capacity of zero creates a no-buffer rendezvous channel. It has no queue
storage: sending waits until a receiver is ready, transfers the value directly,
and then returns `Ok(())`. A waiting receive on a zero-capacity channel returns
`Some(value)` when the paired send transfers a value.
Closing the sender endpoint prevents later sends from succeeding and wakes
waiting receivers.
`channel::select(left, right)` observes two receivers with the same item type.
It returns the first ready value as `Some({index, value})`, using `0` for the
left receiver and `1` for the right receiver, and returns `None` only after
both receivers are closed and drained. If both receivers are ready during one
runtime poll, repeated selections rotate the first polled receiver so that
ties alternate between `0` and `1`.
`channel::select_priority(left, right)` has the same receiver and return
behavior, except ties in one runtime poll always choose the left receiver.
`channel::select_timeout(left, right, timeout_ms)` has the same receiver,
return, and rotating tie-breaking behavior. It also returns `None` when no
value is selected before the non-negative millisecond timeout elapses. A
negative timeout waits without a timeout, matching `channel::select`.
`channel::select_result`, `channel::select_priority_result`, and
`channel::select_timeout_result` use the same readiness, tie-breaking,
closed-channel, and timeout rules as their non-result counterparts. They
return `Ok(Some(selected))` when a receiver produces a value, `Ok(None)` when
selection closes or times out without a value, and `Err(SelectError)` when
cooperative cancellation interrupts the waiting selection.

Task values are backend-owned runtime handles. `task::spawn` starts a
zero-argument callable on a JVM thread and freezes the returned value before it
crosses the task boundary. `task::join` waits for that task and returns
`Ok(value)` on ordinary completion or `Err(JoinError)` on interruption,
cancellation, or runtime failure. `task::cancel` requests cooperative
cancellation by interrupting the task.

File-system intrinsics are backend-owned runtime operations. `fs::read_to_string`
reads UTF-encoded text and returns `Ok(text)` or `Err(FsError)`.
`fs::write_string` writes UTF-encoded text and returns `Ok(())` or
`Err(FsError)`. `fs::exists` returns `Ok(Bool)` for the host existence check or
`Err(FsError)` if the path cannot be interpreted. `fs::read_dir` returns
`Ok(Vec<Path>)` containing backend-owned path values for directory entries or
`Err(FsError)`. These operations use `Result` at the Veln boundary instead of
exposing host exceptions.

Network and time boundary intrinsics are backend-owned runtime operations.
`net::receive_chunk` returns a host-fed immutable `ByteChunk`.
`net::send_chunk` exposes an outgoing immutable `ByteChunk` to the host
runtime and returns `()`. `time::timeout_ms` waits for a non-negative
millisecond duration at the runtime boundary and returns `()`.

Current-process intrinsics are also backend-owned runtime operations.
`process::args` returns the selected entry arguments as a frozen vec of
strings. `process::env` returns `Some(value)` for a present environment key and
`None` for an unavailable key. `process::cwd` returns `Ok(Path)` as a
backend-owned path value for the host current working directory or
`Err(ProcessError)` when the runtime cannot produce one. `process::exit`
terminates the selected host process after clamping the integer status into the
implemented backend status range.

This freeze rule is an observable language boundary only through value
immutability and update semantics. The exact JVM representation, copying
strategy, and later structural-sharing choices remain backend details.

Runtime contract failures stop the selected `run` entry or fail the selected
test case. Human output names the failed clause text, function boundary, source
identity, and blame route. `veln run --json` reports one top-level structured
error record. `veln test --json` embeds runtime contract failures in
the failed case with structured runtime contract details. Entries and tests
that return `Err(value)` are reported with structured runtime result details
in `veln run --json` and `veln test --json`. `require` uses caller blame;
`ensure` uses implementation blame. When `?` propagates an error result
out of a function, the function's `ensure` clauses run before that early
return.

The JVM execution path keeps a persistent class cache for generated JVM
classfile artifacts. Before cached classes are executed, the runner validates a
cache manifest against the emitted class paths and classfile contents expected
for the selected program. Missing manifests, incomplete entries, unexpected
files, and class contents that do not match the expected digest are treated as
invalid cache entries and are regenerated instead of executed. Cache hits may
skip artifact preparation, but command results, stdout, stderr, contract traces,
and captured stdio events are defined as if the selected program was emitted
for that invocation.
