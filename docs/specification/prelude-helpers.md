---
role: specification
authority: normative
update-when: The compiler-known descriptor table, prelude helper surface, or executable prelude-helper evidence changes.
---

# Prelude Helpers

This page specifies compiler-known descriptor metadata and prelude helper behavior.

## Compiler-Known Descriptor Table

Semantic analysis owns a standard symbol table for compiler-known library
symbols. The table records the source-visible module, name, symbol kind, effect
labels, lowering identity, and stability class for the descriptor-backed
subset.

The descriptor-backed subset covers stdio effect metadata, concurrency effect
metadata, and minimal `fs`, `net`, `time`, and `process` intrinsics. The
toolchain `std` package is the source of truth for prelude declarations,
visibility, ordinary types, ADTs other than compiler-owned `Option`, `Result`,
and `List`, and Veln helper bodies. Compiler adapters retain expected-type and
callback inference for public helper names declared by that package.

## Prelude Helpers

Every selected module is checked with an implicit import of `std::prelude`.
The single bootstrap exception is `std::prelude` itself. Other `std` modules
and standard-package tests receive the same-package import.
Prelude helper exports are ordinary pure helper calls for name-resolution
purposes: bare helper names resolve when no local declaration shadows them and
no written import creates an ambiguity, and `prelude::name` selects the
standard helper explicitly. Project analysis resolves those calls to the
package function declaration and lowers reachable Veln bodies as ordinary
functions with collision-resistant internal names. Only calls spelled through
`prelude_builtin::*` inside the package remain prelude intrinsics. The compiler
adapter assigns expected types only to public helper names declared by the
package. Direct low-level surface analysis without project loading retains the
descriptor fallback. Most helpers do not infer effects; explicitly effectful
standard helpers infer the effects declared by their public prelude signature.
No `List`/`Vec` conversion helpers are part of this public helper set; names
such as `list_to_vec` or `vec_to_list` resolve only when user declarations put
them in scope.

`std` is owned by the toolchain. A root package named `std` is accepted only
when its manifest, exports, and non-test sources exactly match the embedded
bundle; extra `*_test.veln` and `.test.veln` files are allowed. Other packages
named `std` and manifest dependencies on `std` report
`manifest.reserved_standard_package`. Explicit imports from package `std`
resolve against the embedded export set, but a source-written alias named
`prelude` remains reserved. The standard package does not participate in
dependency selection and has no lockfile entry.

### Standard Byte ADTs

```veln
type StreamInput
	Chunk(bytes: ByteChunk)
	End
end

type StreamAdapterAction
	SendBytes(bytes: ByteChunk)
	EndStream
	Ignore
end

type AcceptOutcome
	AcceptStream(stream: NetStream)
	AcceptEnd
	AcceptDeadlineExpired
	AcceptCancelled
end

type StreamReadOutcome
	ReadChunk(bytes: ByteChunk)
	ReadEnd
	ReadDeadlineExpired
	ReadCancelled
end

type StreamWriteOutcome
	WriteCompleted
	WriteDeadlineExpired
	WriteCancelled
end

type DecodeError
	DecodeError(id: String, offset: ByteOffset, field_path: String)
	DecodeErrorWithReason(id: String, offset: ByteOffset, field_path: String, reason: String)
end

type DecodeReadiness
	NeedBytes(count: ByteCount)
	NeedEnd
end

type DecodeStep<T>
	Decoded(value: T, consumed: ByteCount)
	NeedMore(readiness: DecodeReadiness)
	Invalid(error: DecodeError)
end

type EncodeError
	EncodeError(id: String, field_path: String, reason: String)
end

type EncodeStep<TState>
	Encoded(chunks: List<ByteChunk>)
	Partial(chunks: List<ByteChunk>, produced: ByteCount, state: TState)
	Invalid(error: EncodeError)
end
```

`StreamInput` is the source-visible incremental input event type. `Chunk`
carries an ordinary immutable `ByteChunk`, including empty chunks, and `End`
is the explicit end-of-stream event.

Source code can model retained pending input by appending incoming
`StreamInput.Chunk` bytes into an immutable `ByteChunk`, checking a
source-owned `ByteCount` limit, taking bounded `ByteView` prefixes for parsing,
dropping consumed bytes, and tracking the next absolute `ByteOffset`
separately for diagnostics. Source code can also collect outgoing immutable
`ByteChunk` values in `List<ByteChunk>` protocol action values without a
separate output chunk type.

`DecodeStep<T>` is the source-visible incremental decode transition type.
`Decoded` carries the decoded value and consumed `ByteCount`, `NeedMore`
carries `DecodeReadiness`, and `Invalid` carries a structured `DecodeError`.
`NeedBytes` names the minimum buffered byte count required before retrying, and
`NeedEnd` represents decoders that need an explicit end-of-stream event.

`EncodeStep<TState>` is the source-visible incremental encode transition
type. `Encoded` carries the complete immutable output chunks, `Partial`
carries committed output chunks, their produced `ByteCount`, and the encoder
state that owns the remaining work, and `Invalid` carries a structured
`EncodeError`. `EncodeError` carries a stable id, source-visible field path,
and representation-failure reason.

`ByteView` is the source-visible bounded immutable byte view. Programs create
checked views with `byte_view(chunk, offset, count)` and inspect the bounded
bytes with the byte-view helper functions; the runtime does not expose a
source-visible borrow lifetime or zero-copy layout guarantee.

### Helper Signatures

```veln
byte(value: Int) -> Result<Byte, String>
byte_to_int(value: Byte) -> Int
byte_chunk(bytes: Vec<Byte>) -> ByteChunk
byte_chunk_count(chunk: ByteChunk) -> ByteCount
byte_append(left: ByteChunk, right: ByteChunk) -> ByteChunk
byte_chunk_from_hex(text: String) -> Result<ByteChunk, String>
byte_chunk_to_visible_ascii_string(chunk: ByteChunk) -> Result<String, String>
byte_chunk_from_visible_ascii_string(text: String) -> Result<ByteChunk, String>
byte_take(chunk: ByteChunk, count: ByteCount) -> Result<ByteChunk, String>
byte_drop(chunk: ByteChunk, count: ByteCount) -> Result<ByteChunk, String>
byte_view(chunk: ByteChunk, offset: ByteOffset, count: ByteCount) -> Result<ByteView, String>
byte_view_to_chunk(view: ByteView) -> ByteChunk
byte_view_count(view: ByteView) -> ByteCount
byte_view_take(view: ByteView, count: ByteCount) -> Result<ByteView, String>
byte_view_drop(view: ByteView, count: ByteCount) -> Result<ByteView, String>
byte_view_slice(view: ByteView, offset: ByteCount, count: ByteCount) -> Result<ByteView, String>
byte_chunks_empty() -> List<ByteChunk>
byte_chunks_one(chunk: ByteChunk) -> List<ByteChunk>
byte_chunks_append(left: List<ByteChunk>, right: List<ByteChunk>) -> List<ByteChunk>
byte_chunks_produce(chunks: List<ByteChunk>, budget: ByteCount) -> {chunks: List<ByteChunk>, produced: ByteCount, remaining: List<ByteChunk>}
byte_read_u8_be(view: ByteView) -> Result<Int, String>
byte_expect_fixed_u8_be(view: ByteView, expected: Int, schema_name: String, field_name: String) -> Result<Int, String>
http2::frame::decode(view: ByteView) -> Result<{length: Int, kind: Int, flags: Int, stream_id: Int, payload: ByteView}, String>
byte_decode_schema_width_sample(view: ByteView) -> Result<{short_value: Int, wide_value: Int}, String>
byte_decode_schema_validation_sample(view: ByteView) -> Result<{length: Int, padding_length: Int}, String>
http2::diagnostic::protocol_closed_with_pending(offset: Int, pending_count: Int, active_continuation: String, expected_stream: Int, started_kind: Int, started_offset: Int, accumulated_header_block_bytes: Int, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::protocol_partial_preface(offset: Int, pending_count: Int, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::protocol_invalid_preface(offset: Int, expected_byte: Int, actual_byte: Int, matched_count: Int, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::protocol_initial_peer_settings_required(offset: Int, actual_kind: Int, actual_flags: Int, stream_id: Int, endpoint_role: String, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::protocol_continuation_expected(offset: Int, actual_kind: Int, actual_stream: Int, expected_stream: Int, started_kind: Int, started_offset: Int, active_continuation: String, accumulated_header_block_bytes: Int, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::protocol_invalid_frame_kind(offset: Int, actual_kind: Int, stream_id: Int, expected_kind: Int, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::protocol_invalid_stream_id(offset: Int, frame_kind: Int, stream_id: Int, required_domain: String, endpoint_role: String, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::protocol_invalid_payload_length(offset: Int, frame_kind: Int, stream_id: Int, observed_length: Int, expected_length: Int, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::protocol_invalid_window_update_increment(offset: Int, stream_id: Int, observed_increment: Int, accepted_min_increment: Int, accepted_max_increment: Int, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::protocol_invalid_request_header_list(offset: Int, frame_kind: Int, stream_id: Int, failed_header_fact: String, header_name: String, decoded_header_names: String, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::protocol_invalid_response_header_list(offset: Int, frame_kind: Int, stream_id: Int, failed_header_fact: String, header_name: String, decoded_header_names: String, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::protocol_content_length_mismatch(offset: Int, frame_kind: Int, stream_id: Int, expected_length: Int, observed_length: Int, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::protocol_settings_not_allowed_for_endpoint(offset: Int, setting_identifier: Int, setting_name: String, endpoint_role: String, frame_kind: Int, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::protocol_invalid_priority_dependency(offset: Int, stream_id: Int, dependency_stream_id: Int, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::protocol_stream_after_goaway(offset: Int, stream_id: Int, last_stream_id: Int, shutdown_state: String, endpoint_role: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::peer_limit_frame_size_exceeded(offset: Int, observed_length: Int, allowed_length: Int, frame_kind: Int, stream_id: Int, receive_limit_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::peer_limit_header_list_size_exceeded(offset: Int, observed_size: Int, allowed_size: Int, frame_kind: Int, stream_id: Int, receive_limit_provenance: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::peer_limit_header_table_size_exceeded(offset: Int, observed_size: Int, allowed_size: Int, frame_kind: Int, stream_id: Int, receive_limit_provenance: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::peer_limit_flow_control_window_exceeded(offset: Int, observed_length: Int, allowed_window_credit: Int, frame_kind: Int, stream_id: Int, active_state: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::peer_limit_concurrent_streams_exceeded(offset: Int, stream_id: Int, attempted_count: Int, allowed_count: Int, endpoint_role: String, active_state: String, receive_limit_provenance: String, rule_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::diagnostic::peer_limit_settings_value_out_of_range(offset: Int, setting_identifier: Int, setting_name: String, observed_value: Int, accepted_min_value: Int, accepted_max_value: Int, peer_limit_provenance: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::hpack::diagnostic::table_size_update_malformed(offset: Int, observed_size: Int, observed_first_byte: Int, expected_fixture: String, codec_module: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::hpack::diagnostic::table_size_update_not_at_start(offset: Int, observed_size: Int, observed_first_byte: Int, observed_update_size: Int, frame_kind: Int, stream_id: Int, active_state: String, expected_fixture: String, codec_module: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
http2::hpack::diagnostic::table_size_update_trailing_bytes(offset: Int, observed_size: Int, observed_first_byte: Int, observed_update_size: Int, frame_kind: Int, stream_id: Int, active_state: String, expected_fixture: String, codec_module: String, preview: ByteView) -> Result<(), RuntimeDiagnostic>
byte_read_u16_be(view: ByteView) -> Result<Int, String>
byte_read_u24_be(view: ByteView) -> Result<Int, String>
byte_read_u31_be(view: ByteView) -> Result<Int, String>
byte_read_u32_be(view: ByteView) -> Result<Int, String>
byte_read_u40_be(view: ByteView) -> Result<Int, String>
byte_read_u48_be(view: ByteView) -> Result<Int, String>
byte_read_u56_be(view: ByteView) -> Result<Int, String>
byte_read_u64_be(view: ByteView) -> Result<Int, String>
byte_read_u16_le(view: ByteView) -> Result<Int, String>
byte_read_u24_le(view: ByteView) -> Result<Int, String>
byte_read_u31_le(view: ByteView) -> Result<Int, String>
byte_read_u32_le(view: ByteView) -> Result<Int, String>
byte_read_u40_le(view: ByteView) -> Result<Int, String>
byte_read_u48_le(view: ByteView) -> Result<Int, String>
byte_read_u56_le(view: ByteView) -> Result<Int, String>
byte_read_u64_le(view: ByteView) -> Result<Int, String>
byte_write_u8_be(value: Int) -> Result<ByteChunk, String>
byte_write_u16_be(value: Int) -> Result<ByteChunk, String>
byte_write_u24_be(value: Int) -> Result<ByteChunk, String>
byte_write_u31_be(value: Int) -> Result<ByteChunk, String>
byte_write_u32_be(value: Int) -> Result<ByteChunk, String>
byte_write_u40_be(value: Int) -> Result<ByteChunk, String>
byte_write_u48_be(value: Int) -> Result<ByteChunk, String>
byte_write_u56_be(value: Int) -> Result<ByteChunk, String>
byte_write_u64_be(value: Int) -> Result<ByteChunk, String>
byte_write_u16_le(value: Int) -> Result<ByteChunk, String>
byte_write_u24_le(value: Int) -> Result<ByteChunk, String>
byte_write_u31_le(value: Int) -> Result<ByteChunk, String>
byte_write_u32_le(value: Int) -> Result<ByteChunk, String>
byte_write_u40_le(value: Int) -> Result<ByteChunk, String>
byte_write_u48_le(value: Int) -> Result<ByteChunk, String>
byte_write_u56_le(value: Int) -> Result<ByteChunk, String>
byte_write_u64_le(value: Int) -> Result<ByteChunk, String>
byte_count(value: Int) -> Result<ByteCount, String>
byte_count_to_int(value: ByteCount) -> Int
byte_offset(value: Int) -> Result<ByteOffset, String>
byte_offset_to_int(value: ByteOffset) -> Int
stream_adapter_drain_actions(stream: NetStream, handler: fn(StreamInput) -> List<StreamAdapterAction>) -> List<StreamAdapterAction> effects [net, concurrency]
stream_adapter_drain_actions_until_cancellable(stream: NetStream, handler: fn(StreamInput) -> List<StreamAdapterAction>, deadline: Deadline, token: CancelToken) -> StreamWriteOutcome effects [net, time, concurrency]
vec_len(items: Vec<A>) -> Int
vec_is_empty(items: Vec<A>) -> Bool
vec_push(items: Vec<A>, value: A) -> Vec<A>
vec_concat(left: Vec<A>, right: Vec<A>) -> Vec<A>
vec_map(items: Vec<A>, f: fn(A) -> B) -> Vec<B>
vec_filter(items: Vec<A>, f: fn(A) -> Bool) -> Vec<A>
vec_fold(items: Vec<A>, initial: B, f: fn(B, A) -> B) -> B
vec_try_map(items: Vec<A>, f: fn(A) -> Result<B, E>) -> Result<Vec<B>, E>
vec_try_map_with(context: C, items: Vec<A>, f: fn(C, A) -> Result<B, E>) -> Result<Vec<B>, E>
list_nil() -> List<A>
list_cons(head: A, tail: List<A>) -> List<A>
list_is_empty(items: List<A>) -> Bool
list_fold(items: List<A>, initial: B, f: fn(B, A) -> B) -> B
list_reverse(items: List<A>) -> List<A>
list_map(items: List<A>, f: fn(A) -> B) -> List<B>
list_filter(items: List<A>, f: fn(A) -> Bool) -> List<A>
list_try_map(items: List<A>, f: fn(A) -> Result<B, E>) -> Result<List<B>, E>
dict_get(dict: Dict<K, V>, key: K) -> Option<V>
dict_contains(dict: Dict<K, V>, key: K) -> Bool
dict_insert(dict: Dict<K, V>, key: K, value: V) -> Dict<K, V>
dict_remove(dict: Dict<K, V>, key: K) -> Dict<K, V>
dict_map(dict: Dict<K, V>, f: fn(K, V) -> A) -> Dict<K, A>
dict_map_with(context: C, dict: Dict<K, V>, f: fn(C, K, V) -> A) -> Dict<K, A>
dict_filter(dict: Dict<K, V>, f: fn(K, V) -> Bool) -> Dict<K, V>
dict_filter_with(context: C, dict: Dict<K, V>, f: fn(C, K, V) -> Bool) -> Dict<K, V>
dict_fold(dict: Dict<K, V>, initial: A, f: fn(A, K, V) -> A) -> A
dict_fold_with(context: C, dict: Dict<K, V>, initial: A, f: fn(C, A, K, V) -> A) -> A
dict_try_map(dict: Dict<K, V>, f: fn(K, V) -> Result<A, E>) -> Result<Dict<K, A>, E>
dict_try_map_with(context: C, dict: Dict<K, V>, f: fn(C, K, V) -> Result<A, E>) -> Result<Dict<K, A>, E>
option_map(value: Option<A>, f: fn(A) -> B) -> Option<B>
option_and_then(value: Option<A>, f: fn(A) -> Option<B>) -> Option<B>
option_unwrap_or(value: Option<A>, fallback: A) -> A
result_map(value: Result<A, E>, f: fn(A) -> B) -> Result<B, E>
result_map_err(value: Result<A, E>, f: fn(E) -> F) -> Result<A, F>
result_and_then(value: Result<A, E>, f: fn(A) -> Result<B, E>) -> Result<B, E>
string_split_once(text: String, separator: String) -> Option<{left: String, right: String}>
string_parse_int(text: String) -> Result<Int, String>
int_to_string(value: Int) -> String
```

The generated schema helper signatures in this list are compatibility and
runtime adapter signatures. Source code should apply schemas through explicit
schema `decode` and `encode` expressions or through ordinary wrapper
functions that call those expressions.

### Value Semantics

Container update helpers return new frozen values and do not mutate their input
containers in place. `vec_len` returns the number of items in the input vec.
`vec_concat` returns a vec containing the left input's items followed by the
right input's items. `vec_is_empty` returns whether a vec contains no items.
`dict_contains` returns true when `dict_get` would return `Some` for the same
dictionary and key, and false when `dict_get` would return `None`.
`dict_map`, `dict_map_with`, `dict_filter`, `dict_filter_with`, `dict_fold`,
`dict_fold_with`, `dict_try_map`, and `dict_try_map_with` visit dictionary
entries in insertion order and pass each key and value to the callback. The
`_with` aliases pass the unchanged context value as the first callback
argument. `dict_map` and `dict_map_with` preserve keys and map values.
`dict_filter` and `dict_filter_with` preserve entries whose callback returns
true. `dict_fold` and `dict_fold_with` thread the accumulator through each
entry. `dict_try_map` and `dict_try_map_with` stop calling their callback
after the first `Err`; otherwise they return `Ok` containing the mapped frozen
dictionary.
`vec_try_map` evaluates items in source order, stops at the first `Err`, and
otherwise returns `Ok` containing the mapped frozen vec in source order.
`vec_try_map_with` follows the same traversal and passes the unchanged context
value as the first callback argument. `vec_map`, `vec_filter`, and `vec_fold`
also visit vec items in source order.
`list_nil` and `list_cons` construct `List` values equivalent to `Nil` and
`Cons`. `list_is_empty` returns true for `Nil` and false for `Cons`.
`list_reverse` returns a list with the input items in reverse order.
`list_map`, `list_filter`, `list_fold`, and `list_try_map` visit list items in
source order. `list_try_map` stops at the first `Err`; otherwise it returns
`Ok` containing the mapped list in source order. List traversal helpers are
implemented without relying on source-level tail-recursion syntax. Public JVM
helper calls for large list traversals do not consume one host stack frame per
list element, and this remains runtime support rather than a general
tail-call optimization guarantee.

`string_split_once` splits at the first occurrence of `separator`, returning
`None` when the separator is absent. `string_parse_int` accepts the backend
integer spelling and returns the original input string in `Err` when parsing
fails. `int_to_string` renders an integer for display and string composition.

`byte(value)` accepts integers from `0` through `255` and returns `Err(String)`
for values outside that range.
`byte_chunk(bytes)` returns an immutable owned
chunk containing the supplied bytes. `byte_chunk_count(chunk)` returns the
chunk length as `ByteCount`. `byte_append(left, right)` returns a new chunk
with the left bytes followed by the right bytes. `byte_chunk_from_hex(text)`
accepts only ASCII hex byte pairs with ASCII whitespace between complete bytes
and returns `Ok(ByteChunk)` for the decoded bytes. It returns `Err(String)`
with `fixture.hex.invalid_character` for non-hex text, prefixes, underscores,
comments, separators, non-ASCII characters, or whitespace inside a byte pair,
and `fixture.hex.odd_length` for a dangling final nibble. The error text
includes the decoded byte offset and the high or low nibble position. When the
error propagates out of `run --json`, the runtime result details expose the
fixture text span, decoded `ByteOffset`, nibble position, and nearby context.
`byte_chunk_to_visible_ascii_string(chunk)` returns `Ok(String)` when every
byte in the chunk is visible ASCII from `0x21` through `0x7e`, preserving byte
order as characters, and returns `Err(String)` for any byte outside that range.
`byte_chunk_from_visible_ascii_string(text)` returns `Ok(ByteChunk)` when every
character is visible ASCII from `0x21` through `0x7e`, preserving character
order as bytes, and returns `Err(String)` for any character outside that range.
`byte_take(chunk, count)` and `byte_drop(chunk, count)` return `Ok(ByteChunk)`
when `count` is within the chunk length, and `Err(String)` when the count is
outside that chunk.
`byte_view(chunk, offset, count)` returns a bounded immutable `ByteView` when
the non-negative offset and count describe a range within the chunk, and
returns `Err(String)` when the range exceeds the chunk length. `byte_view` and
byte reads report negative direct-constructor payloads with the same
non-negative offset and count error strings as the construction helpers.
`byte_view_to_chunk(view)` materializes exactly the bounded bytes as an
immutable owned `ByteChunk`.
Schema-facing byte conversions are ordinary source-visible helper calls, not
implicit schema coercions: source code uses `byte_view` when schema input or
payload fields need bounded `ByteView` values over owned bytes, and
`byte_view_to_chunk` when schema-decoded bounded bytes must be materialized as
an owned `ByteChunk`. The checked cases
`../../examples/specification/run/binary-schema-byte-conversion-boundary/` and
`../../examples/specification/run/binary-schema-byte-conversion-range-json/`
cover the successful boundary and the requested-range failure.
`byte_view_count(view)` returns the view length as `ByteCount`.
`byte_view_take(view, count)`, `byte_view_drop(view, count)`, and
`byte_view_slice(view, offset, count)` derive bounded immutable views within
the supplied view and return `Err(String)` when the requested local range
exceeds the view length. These helpers let source code represent pending input
as bounded `ByteView` values while keeping the absolute `ByteOffset` carried by
the view. `byte_chunks_empty()`, `byte_chunks_one(chunk)`, and
`byte_chunks_append(left, right)` construct and combine `List<ByteChunk>`
values for outgoing chunks without introducing an output-only byte type.
`byte_chunks_produce(chunks, budget)` returns the prefix chunks that fit within
the supplied `ByteCount` budget, the produced byte count, and the remaining
suffix. It preserves chunk order, never splits a `ByteChunk`, returns no
produced chunks for a zero budget, and leaves the remaining suffix unchanged
when the first chunk does not fit.
The fixed-width unsigned big-endian and little-endian read helpers read from
the start of the view and return `Err(String)` when the view is too short.
The `u31` and `u64` reads also return `Err(String)` when the decoded value
would exceed the source-visible `Int` maximum for the helper width. The
source-visible `u56` helpers read and write the same seven-byte big-endian or
little-endian representation as `UInt56be` and `UInt56le`, accept values in
the `0..72057594037927935` range, and reject shorter views or unrepresentable
write values with `Err(String)`.
The
`byte_expect_fixed_u8_be` helper reads one byte and returns
`schema.fixed_field_mismatch` diagnostic details when the actual byte differs
from the expected fixed byte for the supplied schema and field names. The
`byte_decode_schema_width_sample` remains as compatibility coverage for the
narrow executable schema slice for `UInt16be` and `UInt32be`: it reads both
fields from a `ByteView`, returns ordinary `Int` values, and reports schema
truncation with field-path byte diagnostic details. Compatibility binary
schema helper lowering also accepts `UInt16le`,
`UInt24le`, `UInt31le`, `UInt32le`, `UInt40le`, `UInt48le`, `UInt56le`, and
`UInt64le` as little-endian unsigned fields. `UInt40be` uses the matching
five-byte big-endian representation, `UInt48be` uses the matching six-byte
big-endian representation, `UInt56be` uses the matching seven-byte
big-endian representation, and `UInt64be` uses the matching eight-byte
big-endian representation. Those
fields decode to ordinary `Int` when representable and encode with the same
representability boundaries as their matching unsigned widths.
Source `format binary` schema declarations whose fields
all use implemented exact-width unsigned primitives retain compatibility
`byte_decode_<schema>` helpers in their declaring module. Those helpers decode
fields in schema order, check supported field-local `where` predicates after
the owning field is decoded, project field-local fixed equality predicates
through `Err(RuntimeDiagnostic(...))` with `schema.fixed_field_mismatch` when
the decoded value differs, return ordinary `Int` values when validation
passes. Projection from the schema-local record into a domain shape is ordinary
source code at the explicit operation or compatibility helper boundary.
They report `schema.validation_failed` with field path, predicate, decoded
values, and structured byte preview fields when validation fails. The same
eligible schema declarations also retain compatibility
`byte_decode_step_<schema>` helpers that accept `ByteView` plus `ByteOffset`
and return `DecodeStep<T>` with
`Decoded(value, consumed)` for a complete buffered value or
`NeedMore(NeedBytes(count))` for an open view that is too short to decide. The
exact-width, supported reserved-bit, length-bounded `ByteView`, closed
dispatch, extension dispatch, and eligible nested dispatch payload encode
slices retain compatibility
`byte_encode_<schema>` helpers for eligible binary schemas whose
source-visible fields are exact-width unsigned primitives, supported
byte-aligned `ReservedBits(width, value)` fields, the supported
`ReservedBits(1, 0)` before `UInt31be` layout, the supported
general non-byte-aligned `ReservedBits(width, value)` before `UInt8`
byte-prefix layouts whose padded group fits in at most eight big-endian bytes,
supported
prefix `ReservedBits(width, value)` plus `UIntN` layouts whose widths
complete one, two, three, or four big-endian bytes, supported `UIntN` plus
reserved suffix layouts whose widths complete one, two, three, four, five, six,
seven, or eight big-endian bytes, supported visible `UInt8` plus
non-byte-aligned multi-byte `ReservedBits(width, value)` suffix layouts that
fit in one three-byte through eight-byte big-endian storage unit with low
padding, supported `UIntN` plus middle
`ReservedBits(width, value)` plus `UIntN` layouts whose widths complete one,
two, three, or four big-endian bytes, including the narrow two-byte
interleaved middle layout with a sub-byte visible `UIntN`, a reserved field,
`UInt8`, and a final sub-byte visible `UIntN`, supported
`ReservedBits(width, value)` plus two visible sub-byte or byte-width `UIntN`
prefix groups whose widths complete one, two, three, or four big-endian bytes,
supported consecutive non-byte-aligned
`UIntN` and `ReservedBits(width, value)` groups whose widths complete one,
two, three, four, five, six, seven, or eight big-endian bytes,
visible-only packed `UInt1` through `UInt7` groups whose widths complete one,
two, three, four, five, six, seven, or eight big-endian bytes,
bounded `Repeat(count_field, Payload)` fields whose count names an earlier
visible exact-width field and whose payload is an exact-width unsigned
primitive, an eligible nested binary schema, or
`ByteView(length_field)` whose length names an earlier visible exact-width
field,
anonymous record fields whose leaves are exact-width unsigned primitives,
length-bounded
`ByteView(length_field)` fields whose
length names an earlier visible exact-width field,
`ByteView(left_length - right_length)` fields whose operands both name earlier
visible exact-width fields, closed dispatch fields, or extension-tolerant
dispatch fields with earlier visible exact-width tag and length fields. Dispatch
payload cases may be exact-width visible primitive payloads, including
lowercase `uint...` spelling, or eligible nested binary schema
payloads named as earlier same-module binary schemas or public imported binary
schemas through written `use` paths. Those helpers
accept a schema-local visible
record, using ordinary `Int` fields for visible primitives, `ByteView` fields
for length-bounded payloads, `List<ByteView>` fields for repeated bounded
byte-view payloads, and `SchemaDispatchPayload<T>` for extension dispatch
payload fields, and return `Result<ByteChunk, EncodeError>` with field-order
output using each primitive's declared byte order, each supplied byte view's
bounded bytes, or a structured encode error. The
supported reserved-bit encode layout omits byte-aligned
`ReservedBits(width, value)` fields from the value record and writes their
declared fixed values. It also omits `ReservedBits(1, 0)` from the value
record when it immediately precedes `UInt31be`; it omits
`ReservedBits(width, value)` from the value record when it immediately
precedes `UInt8`, the positive width is not byte aligned, the value fits that
width, and the padded group fits in at most eight big-endian bytes. It writes
the declared reserved prefix, visible byte, and trailing zero padding in that
storage group;
supported packed
prefix layouts omit the reserved field and write the declared high bits with
the visible low-bit record field in the shared storage unit. Supported suffix
layouts omit the reserved field and write the visible high-bit record field
with the declared low reserved bits in the shared storage unit. Supported
middle layouts omit the reserved field and write both adjacent visible record
fields around the declared reserved bits in the shared storage unit. Supported
consecutive non-byte-aligned `UIntN` and `ReservedBits(width, value)` groups
omit every reserved field and write visible and declared reserved values in
declaration order in the shared storage unit. The closed
dispatch encode layout selects the payload width from the earlier visible tag
field and reports `schema.dispatch_unknown_tag` when no
case matches. The extension dispatch encode layout writes `Known` selected
payloads, preserves matching unknown raw payload bytes, reports
`schema.dispatch_mismatch` for tag or variant disagreements, and reports
`schema.dispatch_length_mismatch` when the explicit length field differs from
the emitted payload byte count. The fixed-width unsigned big-endian and
little-endian read helpers return `Ok(Int)` when the bounded `ByteView`
contains enough bytes and the decoded unsigned value fits the helper width;
they return `Err(String)` for short views or values larger than the helper
width can represent, such as the 31-bit maximum check. The fixed-width
unsigned big-endian and little-endian write helpers return `Ok(ByteChunk)` for
values in range and `Err(String)` for negative values or values larger than
the helper width can encode.
`byte_count(value)` and `byte_offset(value)` accept non-negative integers.
The `*_to_int` helpers expose the stored integer value for ordinary source
logic and display.

### Standard Package Boundary

The implemented standard symbol table has this current pure-helper split.
This table records compiler-known runtime symbols, including compatibility
helpers, rather than the public schema application surface.

- public `std::prelude` functions with compiler type adapters: `byte`,
  `byte_to_int`, `byte_chunk`,
  `byte_chunk_count`, `byte_append`, `byte_chunk_from_hex`,
  `byte_chunk_to_visible_ascii_string`,
  `byte_chunk_from_visible_ascii_string`, `byte_take`, `byte_drop`,
  `byte_view`, `byte_view_to_chunk`, `byte_view_count`,
  `byte_view_take`, `byte_view_drop`, `byte_view_slice`,
  `byte_chunks_empty`, `byte_chunks_one`, `byte_chunks_append`,
  `byte_chunks_produce`,
  `byte_read_u8_be`,
  `byte_expect_fixed_u8_be`,
  `http2::frame::decode`, `byte_decode_schema_width_sample`,
  `byte_decode_schema_validation_sample`,
  `http2::diagnostic::protocol_closed_with_pending`,
  `http2::diagnostic::protocol_partial_preface`,
  `http2::diagnostic::protocol_invalid_preface`,
  `http2::diagnostic::protocol_initial_peer_settings_required`,
  `http2::diagnostic::protocol_continuation_expected`,
  `http2::diagnostic::protocol_invalid_frame_kind`,
  `http2::diagnostic::protocol_invalid_stream_id`,
  `http2::diagnostic::protocol_invalid_payload_length`,
  `http2::diagnostic::protocol_invalid_window_update_increment`,
  `http2::diagnostic::protocol_invalid_request_header_list`,
  `http2::diagnostic::protocol_invalid_response_header_list`,
  `http2::diagnostic::protocol_content_length_mismatch`,
  `http2::diagnostic::protocol_invalid_priority_dependency`,
  `http2::diagnostic::protocol_stream_after_goaway`,
  `http2::diagnostic::peer_limit_frame_size_exceeded`,
  `http2::diagnostic::peer_limit_header_list_size_exceeded`,
  `http2::diagnostic::peer_limit_header_table_size_exceeded`,
  `http2::diagnostic::peer_limit_flow_control_window_exceeded`,
  `http2::diagnostic::peer_limit_concurrent_streams_exceeded`,
  `http2::diagnostic::peer_limit_settings_value_out_of_range`, `byte_read_u16_be`,
  `byte_read_u24_be`, `byte_read_u31_be`, `byte_read_u32_be`,
  `byte_read_u40_be`, `byte_read_u48_be`, `byte_read_u56_be`,
  `byte_read_u64_be`,
  `byte_read_u16_le`, `byte_read_u24_le`, `byte_read_u31_le`,
  `byte_read_u32_le`, `byte_read_u40_le`, `byte_read_u48_le`,
  `byte_read_u56_le`, `byte_read_u64_le`, `byte_write_u8_be`,
  `byte_write_u16_be`, `byte_write_u24_be`, `byte_write_u31_be`,
  `byte_write_u32_be`, `byte_write_u40_be`, `byte_write_u48_be`,
  `byte_write_u56_be`, `byte_write_u64_be`, `byte_write_u16_le`,
  `byte_write_u24_le`, `byte_write_u31_le`, `byte_write_u32_le`,
  `byte_write_u40_le`, `byte_write_u48_le`, `byte_write_u56_le`,
  `byte_write_u64_le`, `byte_count`, `byte_count_to_int`, `byte_offset`,
  `byte_offset_to_int`,
  `vec_len`, `vec_is_empty`, `vec_push`, `vec_concat`, `vec_map`,
  `vec_filter`, `vec_fold`, `vec_try_map`, `vec_try_map_with`,
  `list_nil`, `list_cons`, `list_is_empty`, `list_fold`, `list_reverse`,
  `list_map`, `list_filter`, `list_try_map`, `dict_get`, `dict_contains`,
  `dict_insert`, `dict_remove`, `dict_map`, `dict_map_with`, `dict_filter`,
  `dict_filter_with`, `dict_fold`, `dict_fold_with`, `dict_try_map`,
  `dict_try_map_with`, `option_map`, `option_and_then`, `option_unwrap_or`,
  `result_map`, `result_map_err`, `result_and_then`, `string_split_once`,
  `string_parse_int`, and `int_to_string`
- compatibility-only float operator adapters remain compiler-owned

The `http2::diagnostic::protocol_invalid_payload_length` helper is a Veln package function
and returns `Result<(), RuntimeDiagnostic>`, matching the source-visible
invalid-payload-length detail used by the HTTP/2 protocol-core fixed
payload-length examples, including `WINDOW_UPDATE`.

The package manifest exports `prelude.veln`; `compiler_support.veln` remains a
private module. The embedded distribution bundle contains every non-test Veln
source exactly once and excludes `*_test.veln` and `.test.veln` files.

The generic runtime diagnostic types are implemented by the private
`std::diagnostic` module. The prelude re-exports `RuntimeDiagnostic`,
`RuntimeDiagnosticDetail`, `RuntimeDiagnosticFieldPathSegment`,
`RuntimeByteDiagnosticFacts`, and `RuntimeBytePreview` as public type aliases,
so established source spelling remains available. HTTP/2 and HPACK protocol
facts use the source-visible inner types `Http2DiagnosticDetail` and
`HpackDiagnosticDetail`. `RuntimeDiagnosticDetail` contains those facts only
through `RuntimeHttp2Diagnostic(...)` and
`RuntimeHttp2HpackDiagnostic(...)`; the concrete protocol constructors belong
to the inner types.

The source-visible diagnostic constructor set includes
`RuntimeValueDiagnostic(...)` for projecting generated binary schema encode
value diagnostics from source-visible `RuntimeDiagnostic(...)` error values,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolClosedWithPendingDiagnostic(...))` for projecting
`http2.protocol.closed_with_pending` failures,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolPartialPrefaceDiagnostic(...))` for projecting
`http2.protocol.partial_preface` failures,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidPrefaceDiagnostic(...))` for projecting
`http2.protocol.invalid_preface` failures,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInitialPeerSettingsRequiredDiagnostic(...))` for projecting
`http2.protocol.initial_peer_settings_required` failures,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolContinuationExpectedDiagnostic(...))` for projecting
`http2.protocol.continuation_expected` failures,
`RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitHeaderListSizeDiagnostic(...))` for projecting
`http2.peer_limit.header_list_size_exceeded` failures,
`RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic(...))` for projecting
`http2.peer_limit.header_table_size_exceeded` failures,
`RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic(...))` for projecting
`http2.peer_limit.concurrent_streams_exceeded` failures,
`RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitSettingsValueDiagnostic(...))` for projecting
`http2.peer_limit.settings_value_out_of_range` failures,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidFrameKindDiagnostic(...))` for projecting
`http2.protocol.invalid_frame_kind` failures from returned diagnostic values,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidStreamIdDiagnostic(...))` for projecting
`http2.protocol.invalid_stream_id` failures from returned diagnostic values,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic(...))` for projecting
`http2.protocol.invalid_data_padding` failures from returned diagnostic
values,
`RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitFlowControlWindowDiagnostic(...))` for projecting
`http2.peer_limit.flow_control_window_exceeded` failures from returned
diagnostic values,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolContentLengthMismatchDiagnostic(...))` for projecting
`http2.protocol.content_length_mismatch` failures from returned diagnostic
values,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic(...))` for projecting
`http2.protocol.invalid_request_header_list` failures from returned diagnostic
values,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic(...))` for projecting
`http2.protocol.invalid_response_header_list` failures from returned
diagnostic values,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic(...))` for
projecting `http2.protocol.invalid_window_update_increment` failures from
returned diagnostic values,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic(...))` for projecting
`http2.protocol.unexpected_settings_ack` failures from returned diagnostic
values,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolSettingsNotAllowedForEndpointDiagnostic(...))` for
projecting `http2.protocol.settings_not_allowed_for_endpoint` failures from
returned diagnostic values,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolPriorityDependencyDiagnostic(...))` for projecting
`http2.protocol.invalid_priority_dependency` failures from returned diagnostic
values, and
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic(...))` for projecting
`http2.protocol.stream_after_goaway` failures from returned diagnostic values.
`http2::diagnostic::protocol_closed_with_pending(...)`,
`http2::diagnostic::protocol_partial_preface(...)`,
`http2::diagnostic::protocol_invalid_preface(...)`,
`http2::diagnostic::protocol_initial_peer_settings_required(...)`,
`http2::diagnostic::protocol_continuation_expected(...)`,
`http2::diagnostic::protocol_invalid_frame_kind(...)`,
`http2::diagnostic::protocol_invalid_stream_id(...)`,
`http2::diagnostic::protocol_invalid_data_padding(...)`,
`http2::diagnostic::protocol_invalid_window_update_increment(...)`,
`http2::diagnostic::protocol_content_length_mismatch(...)`,
`http2::diagnostic::protocol_unexpected_settings_ack(...)`,
`http2::diagnostic::protocol_settings_not_allowed_for_endpoint(...)`,
`http2::diagnostic::protocol_invalid_priority_dependency(...)`,
`http2::diagnostic::protocol_stream_after_goaway(...)`,
`http2::diagnostic::peer_limit_frame_size_exceeded(...)`,
`http2::diagnostic::peer_limit_header_list_size_exceeded(...)`,
`http2::diagnostic::peer_limit_header_table_size_exceeded(...)`,
`http2::diagnostic::peer_limit_flow_control_window_exceeded(...)`,
`http2::diagnostic::peer_limit_concurrent_streams_exceeded(...)`, and
`http2::diagnostic::peer_limit_settings_value_out_of_range(...)`,
`http2::diagnostic::protocol_invalid_request_header_list(...)`, and
`http2::diagnostic::protocol_invalid_response_header_list(...)` return these payloads
directly as `Result<(), RuntimeDiagnostic>`.

Use [Helper Signatures](#helper-signatures) for the implemented signature of
each helper and [Value Semantics](#value-semantics) for behavior. The embedded
source is ordinary Veln source in `std::prelude`. Its public declarations are
the helper admission and visibility boundary, while compiler adapters preserve
expected-type inference and diagnostics at user call sites. Helper bodies and
their reachable private functions lower through the ordinary function path.
Package source may call compiler-known prelude
runtime operations through the reserved `prelude_builtin` module, such as
`prelude_builtin::vec_fold(items, initial, f)`, to avoid spelling a runtime
operation like an ordinary recursive call to the helper being defined.
`vec_len` delegates to `prelude_builtin::vec_len` so the runtime can use the
host vec size directly. The vec traversal helpers use
`prelude_builtin::vec_fold`, and vec append support uses
`prelude_builtin::vec_push`; their step helpers are implementation details, and
this source placement does not expose or stabilize a public vec
representation. Byte hex fixture decoding and byte slice helpers delegate
through `prelude_builtin::byte_chunk_from_hex`,
`prelude_builtin::byte_chunk_to_visible_ascii_string`,
`prelude_builtin::byte_take`, and `prelude_builtin::byte_drop` because text
decoding, visible ASCII conversion, and bounded slicing
currently cross the runtime container boundary. The `vec_fold` entry is
declared in the shared `prelude`
source and delegates to `prelude_builtin::vec_fold`. The list
helpers use the descriptor-backed `List<A>` constructors and pattern coverage;
their private step helpers are ordinary support source and do not expose a
public list representation beyond `Nil` and `Cons`. The dict helpers keep
using the existing prelude runtime operation through
`prelude_builtin::dict_get`, `prelude_builtin::dict_insert`, and
`prelude_builtin::dict_remove`; their public bare names remain ordinary package
function entry points, and `dict_contains` derives its result from the
builtin get operation. Private support functions such as
`vec_try_map_with_step` and `list_try_map_step` are ordinary support source and
are not public compiler adapter entries.

### Compiler-Support Source

The private `std::compiler_support` module contains
`load_source_text(path: Path) -> Result<String, FsError> effects [fs]`. It is
not a prelude export and receives the same-package implicit prelude import. It
exercises Veln source checking and JVM execution through `fs::read_to_string`.

### Diagnostics And Tests

When `vec_map` receives a callback whose return type is `Result`, the checker
reports the ordinary callback type mismatch and adds a repair hint to use
`vec_try_map` for fallible traversal.

The language specification does not promise asymptotic complexity, allocation
counts, representation identity, structural sharing, hashing, or tree-balancing
behavior for these helpers. Those are implementation details until a concrete
container representation is specified. Tests should assert value semantics,
source-order traversal, `Result` short-circuiting, diagnostics, and effect
behavior rather than timings or allocation counts.
