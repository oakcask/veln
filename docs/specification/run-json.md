# Run JSON

`veln run --json` emits schema version `veln-run-json/v0` with:

- `command`: `run`
- `status`: `passed`, `failed`, or `error`
- `exit_code`: the captured Java process exit code, or `1` for tool errors
- `stdout`: captured user-program stdout
- `stderr`: captured user-program stderr
- `error`: `null` for passed runs, or a structured result, runtime, or runner
  error

Runtime contract failures use `error.kind: "contract"`. The error details use
`kind: "contract"` and `phase: "runtime"` and include:

- `clause`: `require`, `ensure`, or `invariant`
- `predicate`: the failed clause text
- `function`: the checked function boundary
- `blame`: `caller` for `require`, `implementation` for `ensure`, and either
  value for `invariant` depending on entry or return failure
- `node_id`: the contract node identifier
- `span`: the source span for the failed clause

Host runtime failures use `error.kind: "runtime"`, `details.phase:
"runtime"`, and the first captured runtime stderr line as `error.message`.
Descriptor-backed transport failures such as malformed host-fed receive bytes,
failed outgoing event recording, fixture-backed socket listen, accept, read,
and write failures, and forced timeout or deadline expiry use this shape.

An entry returning `Err(value)` uses `error.kind: "result"`. The error details
use `kind: "result"`, `phase: "runtime"`, and `value` with the rendered error
value. When the result value is a compact fixture hex failure from
`byte_chunk_from_hex(text)`, `details.fixture_hex` includes:

- `kind: "fixture_hex"`
- `id`: `fixture.hex.invalid_character` or `fixture.hex.odd_length`
- `fixture_text_span`: start and end offsets inside the fixture text
- `byte_offset`: the decoded `ByteOffset` kind and integer value
- `nibble_position`: `high` or `low`
- `nearby_context`: bounded fixture text around the failed span

When the returned error value is
`RuntimeDiagnostic(id, message, RuntimeByteDiagnostic(...))`,
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value and
`details.byte_diagnostic` is projected from that value. The implemented byte
detail constructor
carries the stable id, message, `ByteOffset`, field-path segment list, one of
the supported byte fact constructors, and an optional bounded byte preview.
Count/readiness facts project to `expected_count`, `available_count`, and
`readiness`; range facts project to `requested_count` and `available_count`;
fixed-value facts project to `expected_value` and `actual_value`; reason facts
project to `reason`; and `RuntimeBytePreview` projects to the standard
`byte_preview` object. Plain `Err(value)` values that do not use this
diagnostic ADT remain ordinary result failures with no
`details.byte_diagnostic`. Generated binary schema decode fixed-field
mismatches return this `RuntimeDiagnostic(...)` payload while preserving the
same `schema.fixed_field_mismatch` byte diagnostic shape.

When the returned error value is
`RuntimeDiagnostic(id, message, RuntimeValueDiagnostic(...))` for a generated
binary schema encode failure id such as
`codec.encode_value_unrepresentable`, `details.value` keeps the rendered
`RuntimeDiagnostic(...)` value and `details.value_diagnostic` is projected
from that value. The value detail constructor carries the schema-local field
path segment list and reason text; run JSON derives `field_path`,
`field_path_display`, and `reason` from those fields while keeping the public
`value_diagnostic` shape used by generated `EncodeError(...)` result values.

When the returned error value is
`RuntimeDiagnostic(id, message, RuntimeHpackFixtureDiagnostic(...))`,
`details.value` likewise keeps the rendered `RuntimeDiagnostic(...)` value and
the HPACK fixture detail projects to `details.protocol_diagnostic`. The
unsupported-header-block, malformed-string-length, malformed-raw-string,
malformed-Huffman-padding, Huffman-EOS, and Huffman non-visible fixture
payloads carry byte offset, observed header block size, observed first byte,
expected fixture, codec module, and a bounded header-block byte preview from
the returned error value itself. Dynamic-index fixture payloads use
`RuntimeHpackFixtureDynamicIndexDiagnostic(...)` to add
`requested_dynamic_index` and `dynamic_table_entry_count`. Table-size update
placement payloads use `RuntimeHpackFixtureTableSizeUpdateDiagnostic(...)` to
add `observed_header_table_size`, `frame_kind`, `stream_id`, `stream_ref`,
and `active_state`.
Dynamic-name continuation payloads use
`RuntimeHpackFixtureDynamicNameDiagnostic(...)` to add
`requested_dynamic_index` and `dynamic_table_entry_count` for the focused
missing, malformed, and out-of-range continuation ids.
The standard `hpack_fixture_unsupported_header_block(...)`,
`hpack_fixture_malformed_string_length(...)`,
`hpack_fixture_malformed_raw_string_value(...)`,
`hpack_fixture_malformed_huffman_padding(...)`,
`hpack_fixture_huffman_eos_symbol(...)`,
`hpack_fixture_huffman_non_visible_value(...)`,
`hpack_fixture_dynamic_index_out_of_range(...)`,
`hpack_fixture_dynamic_name_continuation_missing(...)`,
`hpack_fixture_dynamic_name_continuation_malformed(...)`,
`hpack_fixture_dynamic_name_continuation_out_of_range(...)`, and
`hpack_fixture_table_size_update_not_at_start(...)` helpers return their
source-visible HPACK fixture `RuntimeDiagnostic(...)` payloads directly, so
their direct helper examples keep the rendered payload in `details.value` and
project the same HPACK fixture facts into `details.protocol_diagnostic`.

When the returned error value is an HTTP/2 protocol
`RuntimeDiagnostic(...)` payload, `details.value` keeps the rendered
`RuntimeDiagnostic(...)` value and the detail projects to
`details.protocol_diagnostic`. The implemented HTTP/2 payload constructors are
`RuntimeHttp2ProtocolClosedWithPendingDiagnostic(...)` for
`http2.protocol.closed_with_pending`,
`RuntimeHttp2ProtocolPartialPrefaceDiagnostic(...)` for
`http2.protocol.partial_preface`,
`RuntimeHttp2ProtocolInvalidPrefaceDiagnostic(...)` for
`http2.protocol.invalid_preface`,
`RuntimeHttp2ProtocolContinuationExpectedDiagnostic(...)` for
`http2.protocol.continuation_expected`,
`RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic(...)` for
`http2.protocol.unexpected_settings_ack`,
`RuntimeHttp2ProtocolInvalidFrameKindDiagnostic(...)` for
`http2.protocol.invalid_frame_kind`,
`RuntimeHttp2ProtocolInvalidStreamIdDiagnostic(...)` for
`http2.protocol.invalid_stream_id`,
`RuntimeHttp2PeerLimitFrameSizeDiagnostic(...)` for
`http2.peer_limit.frame_size_exceeded`,
`RuntimeHttp2PeerLimitHeaderListSizeDiagnostic(...)` for
`http2.peer_limit.header_list_size_exceeded`,
`RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic(...)` for
`http2.peer_limit.header_table_size_exceeded`,
`RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic(...)` for
`http2.peer_limit.concurrent_streams_exceeded`,
`RuntimeHttp2PeerLimitSettingsValueDiagnostic(...)` for
`http2.peer_limit.settings_value_out_of_range`,
`RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic(...)` for
`http2.protocol.invalid_payload_length`,
`RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic(...)` for
`http2.protocol.invalid_data_padding`,
`RuntimeHttp2PeerLimitFlowControlWindowDiagnostic(...)` for
`http2.peer_limit.flow_control_window_exceeded`,
`RuntimeHttp2ProtocolContentLengthMismatchDiagnostic(...)` for
`http2.protocol.content_length_mismatch`,
`RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic(...)` for
`http2.protocol.invalid_request_header_list`,
`RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic(...)` for
`http2.protocol.invalid_response_header_list`,
`RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic(...)` for
`http2.protocol.invalid_window_update_increment`,
`RuntimeHttp2ProtocolPriorityDependencyDiagnostic(...)` for
`http2.protocol.invalid_priority_dependency`, and
`RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic(...)` for
`http2.protocol.stream_after_goaway`. These constructors project the same
public JSON fields as the compatibility helpers, including stream
classification, peer-limit facts, active state, rule provenance, receive-limit
provenance, header-list facts, decoded header names, and bounded byte previews
where applicable.
The `http2_protocol_closed_with_pending(...)`,
`http2_protocol_partial_preface(...)`,
`http2_protocol_invalid_preface(...)`,
`http2_protocol_continuation_expected(...)`,
`http2_peer_limit_frame_size_exceeded(...)`,
`http2_peer_limit_header_list_size_exceeded(...)`,
`http2_peer_limit_header_table_size_exceeded(...)`,
`http2_peer_limit_concurrent_streams_exceeded(...)`, and
`http2_peer_limit_settings_value_out_of_range(...)` helpers return their
source-visible HTTP/2 `RuntimeDiagnostic(...)` payloads directly, so
`details.value` is the rendered payload instead of a plain string.
The `http2_protocol_invalid_window_update_increment(...)`,
`http2_protocol_content_length_mismatch(...)`,
`http2_protocol_invalid_priority_dependency(...)`,
`http2_protocol_stream_after_goaway(...)`, and
`http2_peer_limit_flow_control_window_exceeded(...)` standard helpers also
return their source-visible HTTP/2 `RuntimeDiagnostic(...)` payloads directly;
their direct helper examples keep the rendered payload in `details.value` and
project the same protocol facts into `details.protocol_diagnostic`.
The `http2_protocol_invalid_stream_id(...)` standard helper likewise returns
the source-visible HTTP/2 `RuntimeDiagnostic(...)` payload directly; its direct
helper example keeps the rendered payload in `details.value` and projects the
same stream id domain facts into `details.protocol_diagnostic`.
The `http2_protocol_invalid_data_padding(...)` and
`http2_protocol_unexpected_settings_ack(...)` standard helpers likewise return
source-visible HTTP/2 `RuntimeDiagnostic(...)` payloads directly; their direct
helper examples keep the rendered payload in `details.value` and project the
same DATA padding or SETTINGS ACK facts into `details.protocol_diagnostic`.
The `http2_protocol_invalid_request_header_list(...)` and
`http2_protocol_invalid_response_header_list(...)` standard helpers likewise
return source-visible HTTP/2 `RuntimeDiagnostic(...)` payloads directly; their
direct helper examples keep the rendered payload in `details.value` and
project the same request or response header-list facts into
`details.protocol_diagnostic`.

When the result value is a closed-input fixed-width `ByteView` read
truncation, `details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "codec.incomplete_input"`
- `byte_offset`: the first missing decoded-stream `ByteOffset`
- `field_path`: schema-local path segment objects with `kind` and `name`;
  empty when no schema owns the read
- `expected_count`: the required byte count
- `available_count`: the byte count available in the view
- `readiness: "need_bytes"`

When the result value is a source-visible `ByteView` range failure,
`details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "codec.byte_range_out_of_bounds"`
- `byte_offset`: the requested `ByteOffset`
- `field_path`: an empty list because no schema owns the range
- `requested_count`: the requested `ByteCount`
- `available_count`: the byte count available from the requested offset
- `byte_preview`: a structured bounded byte preview object for the available
  bytes at that offset

When the result value is a schema fixed-field mismatch,
`details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "schema.fixed_field_mismatch"`
- `byte_offset`: the decoded-stream `ByteOffset` of the mismatched field
- `field_path`: schema-local path segment objects with `kind` and `name`
- `expected_value`: the fixed value required by the schema field
- `actual_value`: the decoded value that was present
- `byte_preview`: a structured bounded byte preview object

When the result value is a binary schema frame-header truncation or generated
binary schema field truncation,
`details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "schema.truncated_field"`
- `byte_offset`: the first missing decoded-stream `ByteOffset`
- `field_path`: schema-local path segment objects with `kind` and `name`
- `expected_count`: the required field byte count
- `available_count`: the byte count available for that field
- `readiness: "need_bytes"`
- `byte_preview`: a structured bounded byte preview object

For repeated binary schema fields, the same `schema.truncated_field` shape
adds an `index` segment after the repeated field segment in `field_path`; the
segment `name` is the zero-based element index whose representation could not
be fully read. Nested repeated schema failures append the nested schema field
segments after that `index` segment.
Packed visible-only sub-byte groups use the same shape and report the first
field in the group when the shared byte is missing.

When the result value is a binary schema payload length boundary failure,
`details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "schema.length_out_of_bounds"`
- `byte_offset`: the first missing decoded-stream `ByteOffset`
- `field_path`: schema-local path segment objects with `kind` and `name`
- `expected_count`: the decoded payload length
- `available_count`: the byte count available after the frame header
- `byte_preview`: a structured bounded byte preview object

When the result value is a binary schema repeat count or byte-view length
division-by-zero failure, `details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "schema.length_division_by_zero"`
- `byte_offset`: the decoded-stream `ByteOffset` where the repeat or
  byte-view field length/count is evaluated
- `field_path`: schema-local path segment objects with `kind` and `name`
- `length_expression`: the repeat count or byte-view length expression
- `divisor_operand`: the right operand whose decoded value was zero
- `operator: "/"`
- `byte_preview`: a structured bounded byte preview object

When the result value is a binary schema integer range failure,
`details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "schema.integer_out_of_range"`
- `byte_offset`: the decoded-stream `ByteOffset` of the field whose decoded
  integer exceeds the schema-owned external integer range
- `field_path`: schema-local path segment objects with `kind` and `name`
- `byte_width`: the decoded field byte width
- `min_value`: the smallest representable value for the schema field
- `max_value`: the largest representable value for the schema field
- `actual_value`: the decoded integer value that was present
- `byte_preview`: a structured bounded byte preview object

When the result value is a binary schema reserved-bit mismatch,
`details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "schema.reserved_bits_mismatch"`
- `byte_offset`: the decoded-stream `ByteOffset` of the reserved field
- `field_path`: schema-local path segment objects with `kind` and `name`
- `bit_width`: the reserved bit width
- `expected_value`: the fixed bit pattern required by the schema field
- `actual_value`: the decoded bit pattern that was present
- `byte_preview`: a structured bounded byte preview object

Packed reserved bitfield groups use the same projection at the specific
reserved field that mismatched. The checked
`examples/specification/run/binary-schema-general-reserved-bitfield-json/`
case covers a general two-byte bitfield group with more than one
representation-only reserved field.

When the result value is a binary schema field-local or schema-level
validation failure,
`details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "schema.validation_failed"`
- `byte_offset`: the decoded-stream `ByteOffset` of the field that owns the
  failed field-local predicate, or the offset after the decoded schema body
  for a failed schema-level predicate
- `field_path`: schema-local path segment objects with `kind` and `name`
- `predicate`: the failed field-local `where` or schema-level `validate`
  predicate text
- `field_value`: the decoded value of the owning field for field-local
  validation failures
- `decoded_values`: display text for decoded schema fields available to the
  predicate
- decoded field values available to the predicate, keyed by schema field name,
  such as `length` and `padding_length`
- `byte_preview`: a structured bounded byte preview object

When the result value is a binary schema mapping division-by-zero failure,
`details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "schema.mapping_division_by_zero"`
- `byte_offset`: the offset after the decoded schema body
- `field_path`: schema-local path segment objects with `kind` and `name`,
  ending at the mapping target field
- `target_field`: the target field whose mapping expression failed
- `operator: "/"`
- `byte_preview`: a structured bounded byte preview object

When the result value is a schema value validation failure from
`validate_<schema>`, `details.value_diagnostic` includes:

- `kind: "value_diagnostic"`
- `id: "schema.validation_failed"`
- `field_path`: schema-local path segment objects with `kind` and `name`
- `predicate`: the failed field-local `where` predicate text
- `field_value`: the supplied value of the owning field
- `supplied_values`: display text for supplied decoded schema `Int` fields
  available to the predicate
- supplied decoded `Int` values available to the predicate, keyed by schema
  field name, such as `length` and `padding_length`

When the result value is a checked `byte_write_*` conversion failure,
`details.value_diagnostic` includes:

- `kind: "value_diagnostic"`
- `id: "codec.byte_write_value_unrepresentable"`
- `field_path`: an empty list because source-visible byte write helpers are
  not owned by a schema field
- `helper_name`: the source-visible byte write helper name
- `supplied_value`: the `Int` value supplied to the helper
- `min_value`: the smallest accepted value
- `max_value`: the largest accepted value
- `width`: the write width in bytes
- `byte_order`: `big_endian` or `little_endian`

When the result value is a source-visible
`EncodeError(id, field_path, reason)` with a supported generated encode
diagnostic id, or a `veln run` entry returns
`EncodeStep::Invalid(EncodeError(id, field_path, reason))`, or the entry
returns `Err(RuntimeDiagnostic(id, message, RuntimeValueDiagnostic(field_path,
reason)))` for the same generated encode ids,
`details.value_diagnostic` includes:

- `kind: "value_diagnostic"`
- `id`: one of `codec.encode_value_unrepresentable`,
  `codec.encode_mapping_mismatch`, `codec.dispatch_unknown_tag`,
  `codec.dispatch_length_mismatch`, or
  `codec.dispatch_mismatch`, or `schema.validation_failed`
- `field_path`: schema-local path segment objects with `kind` and `name`,
  derived from the source-visible field path
- `field_path_display`: the source-visible field path string for
  representation, dispatch, and mapping failures
- `reason`: the source-visible encode failure reason for representation,
  dispatch, and mapping failures
- `expected_count`, `actual_count`, `length_expression`, `byte_offset`, and
  `byte_preview` for generated length-bounded `ByteView` encode count
  mismatches
- `predicate`, `field_value`, `supplied_values`, and supplied schema-local
  `Int` values keyed by field name for encode-time `schema.validation_failed`

`EncodeStep::Encoded(...)` and `EncodeStep::Partial(...)` entry results do not
populate `error` or `details.value_diagnostic`.

When a `veln run` entry returns a source-visible
`DecodeError(id, byte_offset, field_path)`,
`DecodeErrorWithReason(id, byte_offset, field_path, reason)`,
`DecodeStep::Invalid(DecodeError(id, byte_offset, field_path))`, or
`DecodeStep::Invalid(DecodeErrorWithReason(id, byte_offset, field_path, reason))`,
`details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id`: the source-visible decode error id
- `byte_offset`: the source-visible `ByteOffset`
- `field_path`: schema-local path segment objects with `kind` and `name`,
  derived from the source-visible field path
- `field_path_display`: the source-visible field path string
- `reason`: the source-visible decode failure reason when the value is
  `DecodeErrorWithReason`
- `local_byte_offset`: the byte offset reported by helper context carried by
  the reason when present
- `expected_count`: the byte count expected by helper context carried by the
  reason when present
- `available_count`: the byte count available to helper context carried by the
  reason when present
- `byte_preview`: a structured bounded byte preview object for helper context
  carried by the reason when present

This shape also covers codec-owned invalid-input facts returned by a
hand-written `decode with` codec boundary, such as `codec.invalid_input` and
`codec.packet_kind_invalid`, and direct `Result<_, DecodeError>` failures
that carry those codec-owned ids. A plain `DecodeErrorWithReason` reason is
kept only as `reason`; helper-only fields are omitted unless the reason
matches registered helper context. If the reason is a byte-helper failure
message with registered helper context, the command-facing projection keeps
the reason text and adds the carried helper counts, local byte offset, and
byte preview to the same `details.byte_diagnostic`.

The checked `codec.consumed_count_invalid` command-facing slice comes from a
hand-written `decode with` codec boundary whose returned `Decoded` consumed
count is outside the supplied `ByteView`. It uses the same
`DecodeStep::Invalid(DecodeError(...))` JSON shape and does not set
`readiness`, because the supplied bytes already prove the consumed-count fact
false.

`DecodeStep::NeedMore(NeedBytes(count))` entry results are reported as
closed-input codec incomplete-input failures. `details.byte_diagnostic`
includes:

- `kind: "byte_diagnostic"`
- `id: "codec.incomplete_input"`
- `byte_offset`: the requested buffered byte count as a `ByteOffset`
- `field_path`: an empty list because source-visible `NeedMore` carries no
  schema field path
- `readiness: "need_bytes"`
- `needed_count`: the source-visible `ByteCount`

`DecodeStep::NeedMore(NeedEnd)` uses the same diagnostic id and readiness
field without `needed_count`. The checked
`examples/specification/run/codec-decode-decoded-json/` case covers that
`DecodeStep::Decoded(...)` entry results do not populate `error` or
`details.byte_diagnostic`.

When the result value is a binary schema closed dispatch unknown tag failure,
`details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "schema.dispatch_unknown_tag"`
- `byte_offset`: the decoded-stream `ByteOffset` of the dispatch field
- `field_path`: schema-local path segment objects with `kind` and `name`
- `tag_field`: the schema-local field name used for dispatch
- `decoded_tag_value`: the decoded integer tag value
- `expected_tags`: display text for the closed set of accepted tag values
- `byte_preview`: a structured bounded byte preview object

For schema-owned byte diagnostics, `byte_preview` includes:

- `encoding: "hex"`
- `data`: lowercase hex byte pairs for the previewed bytes, with no
  separators
- `preview_byte_count`: the number of bytes present in `data`
- `total_byte_count`: the total byte count represented by the diagnostic byte
  source
- `truncated`: whether `data` is a shortened prefix of the preview source

Named binary fixture cases can assert the same byte-stream facts after a
fixture record decodes successfully. Invalid compact hex remains a
`details.fixture_hex` failure. Valid fixture bytes that are too short for a
closed-input read remain ordinary codec truncation without fixture hex details.
Valid fixture bytes that fail a test-owned codec or protocol field check use
fixture metadata for the diagnostic id, byte offset, structured field path,
and consumed count where applicable.

HTTP/2 protocol-core failures are ordinary source-level error ADT values until
a command, fixture helper, or adapter explicitly reports them through the
source-visible `http2_protocol_diagnostic` projection function. Returning a
protocol error value by itself does not attach `details.protocol_diagnostic`
or emit a human diagnostic. The projection function accepts the protocol error
plus `Http2DiagnosticContext`, then routes each supported failure shape to the
stable helper that owns the diagnostic id, primary message, related notes, and
structured details. The HTTP/2 protocol-core executable example and the
converted command-facing frame-size, header-list-size, invalid-frame-kind,
stream-id, post-GOAWAY, payload-length, DATA-padding, content-length,
SETTINGS ACK, preface, continuation, and priority self-dependency examples
check this boundary for both `http2.peer_limit.*` and `http2.protocol.*`
failures.
Accepted HTTP/2 send-intents, including outbound HEADERS output split across
HEADERS and CONTINUATION frames and server-side outbound `PUSH_PROMISE`
output split across `PUSH_PROMISE` and CONTINUATION frames, remain ordinary
program stdout in the
aggregate protocol-core run case; they do not populate `error` or
`details.protocol_diagnostic`. The same applies when those send-intents build
their opaque header-block bytes from fixture header-list values through the
HPACK fixture encoder, including checked Huffman-marked string literal
fixtures for outbound HEADERS and `PUSH_PROMISE`, and including the checked
stateful `PUSH_PROMISE` path where the returned fixture encode state lets a
later promised header list use the dynamic indexed byte `0xbe`.

HTTP/2 protocol-core failures that originate from a source-visible
`RuntimeDiagnostic(...)` payload attach `details.protocol_diagnostic`.
End-of-stream with a partial client connection preface uses id
`http2.protocol.partial_preface` and records
`byte_offset.value`, `pending_count`, `expected_count`, `active_state`, and
`rule_provenance`, plus a structured bounded `byte_preview` for the pending
raw input bytes. A mismatched client connection preface byte uses id
`http2.protocol.invalid_preface` and records `byte_offset.value`,
`expected_byte`, `actual_byte`, `matched_prefix_count`, `expected_count`,
`active_state`, and `rule_provenance`, plus a structured bounded
`byte_preview` for the raw input bytes inspected by the preface check. In both
preface cases, `details.value` keeps the rendered `RuntimeDiagnostic(...)`
value with `RuntimeHttp2ProtocolPartialPrefaceDiagnostic(...)` or
`RuntimeHttp2ProtocolInvalidPrefaceDiagnostic(...)`. Input
end with pending bytes after the preface uses id
`http2.protocol.closed_with_pending` and records `byte_offset.value`,
`pending_count`, `input_event`, and `active_continuation`, plus a structured
bounded `byte_preview` for the retained pending bytes. A frame that violates
an active header-block continuation sequence uses id
`http2.protocol.continuation_expected` and records `byte_offset.value`,
`actual_frame_kind`, `actual_stream_id`, `expected_stream_id`,
`started_frame_kind`, `started_byte_offset`, and `active_continuation`, plus a
structured bounded `byte_preview` for the inspected incoming frame header
bytes. These protocol-owned byte previews use the same `encoding`, `data`,
`preview_byte_count`, `total_byte_count`, and `truncated` object shape as
schema-owned byte diagnostics, while byte offset, expected byte count, actual
pending count, matched prefix count, expected byte, actual byte, active
protocol state, active continuation state, and rule provenance stay in their
own fields. The checked closed-input pending-byte and continuation-ordering
JSON examples return source-visible
`RuntimeHttp2ProtocolClosedWithPendingDiagnostic(...)` and
`RuntimeHttp2ProtocolContinuationExpectedDiagnostic(...)` payloads, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields. The
`http2_protocol_closed_with_pending`,
`http2_protocol_continuation_expected`,
`http2_peer_limit_frame_size_exceeded`,
`http2_peer_limit_header_table_size_exceeded`, and
`http2_peer_limit_concurrent_streams_exceeded` standard helpers return the
same `RuntimeDiagnostic(...)` values directly, so their JSON result details
are also derived from the returned value.
The frame-size
peer-limit slice uses id
`http2.peer_limit.frame_size_exceeded` and records
`byte_offset.value`, `observed_payload_length`, `allowed_max_frame_size`,
`frame_kind`, `stream_id`, `stream_ref`, `receive_limit_provenance`, and a
structured bounded `byte_preview` for the inspected incoming frame header. The
preview uses the same object shape as other protocol-owned byte previews. The
provenance names the active receive-limit entry used for the failed inbound
frame-size check, such as protocol default, local configuration, or local
SETTINGS. Peer-received `SETTINGS_ENABLE_PUSH`, `SETTINGS_MAX_FRAME_SIZE`,
`SETTINGS_MAX_CONCURRENT_STREAMS`, `SETTINGS_INITIAL_WINDOW_SIZE`,
`SETTINGS_HEADER_TABLE_SIZE`, and `SETTINGS_MAX_HEADER_LIST_SIZE` values
belong to peer-advertised state for outbound decisions and are not reported as
the receive-limit provenance for later inbound frame-size, concurrent-stream,
or DATA receive-window failures. A received `SETTINGS_INITIAL_WINDOW_SIZE`
delta can still change the tracked open stream's allowed receive-window
credit, and later DATA failures report that adjusted credit.
Unknown SETTINGS identifiers do not update peer-advertised state and do not produce
`http2.peer_limit.settings_value_out_of_range`; known SETTINGS items in the
same frame are still applied or diagnosed at their own item byte offset.
Received DATA frames that exceed available
inbound receive-window credit, and
`WINDOW_UPDATE` increments that would exceed available inbound receive-window
growth, use id
`http2.peer_limit.flow_control_window_exceeded` and record
`byte_offset.value`, `observed_payload_length`, `allowed_window_credit`,
`frame_kind`, `stream_id`, `stream_ref`, `active_state`, and
`rule_provenance`, plus a structured bounded `byte_preview` for the inspected
payload bytes. The checked HTTP/2 examples cover both stream-window and
connection-window DATA receive credit failures through source-visible
`RuntimeHttp2PeerLimitFlowControlWindowDiagnostic(...)` payloads, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields.
Zero received `WINDOW_UPDATE` increments use id
`http2.protocol.invalid_window_update_increment` and record
`byte_offset.value`, `frame_kind`, `stream_id`, `stream_ref`,
`observed_window_increment`, `accepted_min_window_increment`,
`accepted_max_window_increment`, `active_state`, `rule_provenance`, and a
structured bounded `byte_preview` for the inspected four-byte increment
payload. The preview uses the same object shape as other protocol-owned byte
previews while byte offset, frame identity, stream identity, decoded
increment, accepted range, active state, and rule provenance remain separate
fields. Invalid
PADDED DATA uses id `http2.protocol.invalid_data_padding` and records
`byte_offset.value`, `frame_kind`, `stream_id`, `stream_ref`, `pad_length`,
`remaining_payload_length`, `active_state`, `rule_provenance`, and a
structured bounded payload byte preview. Accepted `content-length`
body-length mismatches use id
`http2.protocol.content_length_mismatch` and record `byte_offset.value`,
`frame_kind`, `stream_id`, `stream_ref`, `expected_content_length`,
`observed_body_length`, `active_state`, `rule_provenance`, and a bounded DATA
application-byte preview. The checked run examples cover both over-length DATA
and an early peer `END_STREAM` shortfall through source-visible
`RuntimeHttp2ProtocolContentLengthMismatchDiagnostic(...)` payloads. A
peer-created stream that would exceed the active concurrent-stream receive
limit uses id `http2.peer_limit.concurrent_streams_exceeded` and records
`byte_offset.value`, `stream_id`, `stream_ref`,
`current_open_peer_created_stream_count`,
`attempted_concurrent_stream_count`, `allowed_concurrent_stream_count`,
`endpoint_role`, `active_state`, `receive_limit_provenance`, and
`rule_provenance`, plus a structured bounded `byte_preview` for the inspected
HEADERS frame header bytes. Received
HEADERS or a completed CONTINUATION header block whose fixture-decoded header
list size exceeds the active local receive policy uses id
`http2.peer_limit.header_list_size_exceeded` and records
`byte_offset.value`, `observed_header_list_size`,
`allowed_header_list_size`, `frame_kind`, `stream_id`, `stream_ref`,
`receive_limit_provenance`, `rule_provenance`, and a structured bounded
`byte_preview` for the inspected header-block bytes; peer-advertised
`SETTINGS_MAX_HEADER_LIST_SIZE` remains outbound peer state and is not used as
the receive-limit provenance for rejecting incoming header blocks. A decoded
HPACK dynamic table-size update in received HEADERS or a final CONTINUATION
whose requested size exceeds the active local header-table receive policy uses
id `http2.peer_limit.header_table_size_exceeded` and records
`byte_offset.value`, `observed_header_table_size`,
`allowed_header_table_size`, `frame_kind`, `stream_id`, `stream_ref`,
`receive_limit_provenance`, `rule_provenance`, and a structured bounded
`byte_preview` for the inspected header-block bytes; peer-advertised
`SETTINGS_HEADER_TABLE_SIZE` remains outbound peer state and is not reported
as the receive-limit provenance for rejecting incoming table-size updates.
The `http2_peer_limit_header_table_size_exceeded(...)` standard helper
returns the same `RuntimeDiagnostic(...)` value directly, so its JSON result
details are also derived from the returned value.
The `http2_peer_limit_concurrent_streams_exceeded(...)` standard helper
likewise returns the same `RuntimeDiagnostic(...)` value directly, preserving
`details.value` and the structured concurrent-stream protocol fields from the
returned payload, including the inspected frame-header byte preview.
Received
request header-list validation failures use id
`http2.protocol.invalid_request_header_list` and record
`byte_offset.value`, `frame_kind`, `stream_id`, `stream_ref`,
`failed_header_fact`, `header_name`, `decoded_header_names`,
`byte_preview`, `active_state`, and `rule_provenance`. The bounded
`byte_preview` records the inspected header-block bytes. The checked
projections cover a
missing required request pseudo-header, a response-only `:status`
pseudo-header, a duplicate request pseudo-header, a request pseudo-header
after a regular header, an uppercase ordinary header name, and an ordinary
header name outside the HTTP field-name token shape, plus a
connection-specific ordinary header name and invalid `te` value on an inbound
request, and invalid and mismatched `content-length` values; the larger
protocol-core fixture also checks the integrated completed HEADERS and final
CONTINUATION paths, including accepted `te: trailers` and accepted
`content-length` values. The focused request header-list JSON examples,
including the raw HPACK uppercase and invalid-token trailer-name projections,
return
source-visible `RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic(...)`
payloads so `details.value` preserves the rendered `RuntimeDiagnostic(...)`
value while `details.protocol_diagnostic` keeps the same public fields,
including the header-block preview. The focused `PUSH_PROMISE` promised
request-header helper JSON example keeps the same projection shape while
preserving frame kind `5` and the promised request header-block preview.
Received response header-list validation failures use id
`http2.protocol.invalid_response_header_list` and record the same structured
fields, including `byte_preview` for the inspected header-block bytes. The
checked projections cover a missing required `:status`, duplicate
`:status`, request-only pseudo-headers, and a response pseudo-header after a
regular header, an uppercase ordinary header name, and an ordinary header
name outside the HTTP field-name token shape, plus invalid `te` value human
output and invalid and mismatched `content-length` values; the larger
protocol-core fixture also checks `:authority` as request-only and checks
completed HEADERS and final CONTINUATION paths. The focused response
header-list human and JSON examples return source-visible
`RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic(...)` payloads, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields, including the
header-block preview. The larger
protocol-core fixture also checks valid ordinary response header lists,
including accepted `te: trailers` and accepted `content-length` values,
through integrated completed HEADERS and final CONTINUATION paths.
Received
SETTINGS range failures use id
`http2.peer_limit.settings_value_out_of_range` and record
`byte_offset.value`, `setting_identifier`, `setting_name`, `observed_value`,
`accepted_min_value`, `accepted_max_value`, `peer_limit_provenance`, and a
structured bounded `byte_preview` for the offending six-byte SETTINGS item.
The preview uses the same object shape as other protocol-owned byte previews
while byte offset, setting identity, observed value, accepted range, and
peer-limit provenance remain separate fields. The
`http2_peer_limit_settings_value_out_of_range(...)` standard helper returns
the same `RuntimeDiagnostic(...)` value directly, so its JSON result details
are also derived from the returned value. The
stream id domain slice uses id `http2.protocol.invalid_stream_id` and records
`byte_offset.value`, `frame_kind`, `stream_id`, `stream_ref`,
`required_stream_id_domain`, `endpoint_role`, `active_state`, and
`rule_provenance`, plus a structured bounded `byte_preview` for the inspected
frame-header bytes. Source-visible
`RuntimeHttp2ProtocolInvalidStreamIdDiagnostic(...)` payloads keep the
rendered `RuntimeDiagnostic(...)` in `details.value` while projecting the same
protocol diagnostic fields. The standard
`http2_protocol_invalid_stream_id(...)` helper returns the same
`RuntimeDiagnostic(...)` value directly, so its JSON result details are also
derived from the returned value.
The preview uses the same object shape as other protocol-owned byte previews
while stream id domain facts stay in their own fields; the checked HTTP/2
examples cover invalid zero stream ids, even client stream ids, nonzero stream
ids on connection-only frames, and CONTINUATION on the connection stream while
a nonzero-stream header block is pending. The
invalid frame-kind state slice uses id `http2.protocol.invalid_frame_kind` and
records `byte_offset.value`, `actual_frame_kind`, `stream_id`, `stream_ref`,
`expected_frame_kind`, `active_state`, and `rule_provenance`, plus a
structured bounded `byte_preview` for the inspected frame header bytes. The
preview uses the same object shape as other protocol-owned byte previews while
frame-kind and stream-state facts stay in their own fields; the checked HTTP/2
examples cover connection-control, idle-stream, reset-stream, and
closed-by-peer stream state failures, plus peer-sent `PUSH_PROMISE` rejection
on a nonzero stream and DATA on a reserved-by-peer promised stream before
its response HEADERS block. The direct standard helper connection-level and
stream-level JSON examples, closed-by-peer stream-state example, peer-sent
`PUSH_PROMISE` JSON examples, and promised-stream DATA-before-HEADERS JSON
example return source-visible
`RuntimeHttp2ProtocolInvalidFrameKindDiagnostic(...)` payloads, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields.
The broad HTTP/2 protocol-core run example also fixes ordinary stdout evidence
for client-side peer-sent `PUSH_PROMISE` receive: accepted single-frame and
final-CONTINUATION cases expose the stripped promised header block as checked
lowercase hex output and print the reserved-by-peer promised stream state.
It validates the decoded promised request header list before reservation,
including accepted ordinary request headers and rejected `:status` and invalid
`te` request-header facts through the existing request header-list diagnostic
shape. It also accepts the first response HEADERS block on that promised stream,
checks the open and `END_STREAM` closed-by-peer lifecycle outcomes, and keeps
DATA before that response HEADERS block on the same invalid frame-kind
diagnostic boundary. Stream id zero, promised stream id zero, wrong
promised-stream parity, wrong associated-stream parity, short payload, and
unsupported HPACK fixture
inputs keep their existing structured diagnostic shapes inside the
`run --json` stdout envelope.
After receiving GOAWAY or after locally sending GOAWAY, a peer-created
HEADERS stream or local outbound HEADERS send-intent greater than the recorded
last stream id uses id `http2.protocol.stream_after_goaway` and records
`byte_offset.value`, `stream_id`, `stream_ref`, `last_stream_id`,
`shutdown_state`, `endpoint_role`, `byte_preview`, `active_state`, and
`rule_provenance`. The peer-created receive case carries a bounded inspected
frame-header preview; the local outbound helper form can carry an empty
preview when no peer bytes were inspected.
The checked `run --json` protocol-core example also keeps already-admitted
peer-created stream DATA and trailer HEADERS after received GOAWAY as passed
stdout, not as a `protocol_diagnostic`; receive-window credit, HPACK fixture
decode, and closed-by-peer lifecycle facts remain ordinary executable output.
The checked stream-after-GOAWAY human and JSON examples return
source-visible `RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic(...)`
payloads for peer-created and local outbound HEADERS streams, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields and structured
preview object.
Wrong-length protocol payloads use id
`http2.protocol.invalid_payload_length` and record `byte_offset.value`,
`frame_kind`, `stream_id`, `stream_ref`, `observed_payload_length`,
`expected_payload_length`, `active_state`, and `rule_provenance`, plus a
structured bounded `byte_preview` for the inspected payload bytes. The preview
uses the same object shape as other protocol-owned byte previews while
payload length facts stay in their own fields. The SETTINGS ACK, PING,
GOAWAY, `RST_STREAM`, and `WINDOW_UPDATE` checked JSON examples return
source-visible
`RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic(...)` payloads, and the
`http2_protocol_invalid_payload_length(...)` helper returns the same
source-visible payload directly. The checked helper JSON example also covers
the `WINDOW_UPDATE` fixed payload-length case with frame kind 8, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields. PADDED DATA
failures use source-visible
`RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic(...)` payloads, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields, including the
structured bounded byte preview. The `http2_protocol_invalid_data_padding(...)`
standard helper returns the same source-visible payload directly. A SETTINGS ACK
received while no local SETTINGS batch is
outstanding uses id `http2.protocol.unexpected_settings_ack` and records
`byte_offset.value`, `frame_kind`, `stream_id`, `stream_ref`, `active_state`,
and `rule_provenance`, plus a structured bounded `byte_preview` for the
inspected frame header bytes. The preview uses the same object shape as other
protocol-owned byte previews while SETTINGS ACK state facts stay in their own
fields. The checked human and JSON examples return a source-visible
`RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic(...)` payload, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields. The
`http2_protocol_unexpected_settings_ack(...)` standard helper returns the same
source-visible payload directly. A PRIORITY frame
whose dependency stream id is its own
stream id uses id `http2.protocol.invalid_priority_dependency` and records
`byte_offset.value`, `frame_kind`, `stream_id`, `stream_ref`,
`dependency_stream_id`, `active_state`, and `rule_provenance`, plus a
structured bounded `byte_preview` for the inspected PRIORITY payload bytes.
The checked human and JSON examples return a source-visible
`RuntimeHttp2ProtocolPriorityDependencyDiagnostic(...)` payload, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields.
The HPACK fixture boundary uses id `hpack.fixture.unsupported_header_block`
for unsupported header blocks, `hpack.fixture.malformed_string_length` for
malformed HPACK string-length encodings,
`hpack.fixture.malformed_raw_string_value` for malformed raw string values on
supported literal-name forms,
`hpack.fixture.malformed_huffman_padding` for malformed Huffman padding,
`hpack.fixture.huffman_eos_symbol` for HPACK Huffman EOS decoded as a
symbol, and `hpack.fixture.huffman_non_visible_value` for HPACK Huffman output
that decodes to a non-visible checked header value. The source-visible runtime
diagnostic payload path for those ids uses the same
`details.protocol_diagnostic` field shape as the standard fixture helpers.
These diagnostics record
`byte_offset.value`, `observed_header_block_size`,
`observed_first_byte`, `expected_fixture`, and `codec_module`, plus a
structured bounded `byte_preview` for the inspected header-block bytes.
Dynamic indexed lookup failures use id
`hpack.fixture.dynamic_index_out_of_range` and also record
`requested_dynamic_index` and `dynamic_table_entry_count` before the same
expected fixture, codec module, and bounded byte-preview fields. Source-visible
payloads for this id carry those fields in
`RuntimeHpackFixtureDynamicIndexDiagnostic(...)`.
Missing, malformed, and out-of-range dynamic-name continuations use ids
`hpack.fixture.dynamic_name_continuation_missing`,
`hpack.fixture.dynamic_name_continuation_malformed`, and
`hpack.fixture.dynamic_name_continuation_out_of_range`; their source-visible
payloads carry the same requested dynamic index, dynamic table entry count,
expected fixture, codec module, and bounded byte-preview fields in
`RuntimeHpackFixtureDynamicNameDiagnostic(...)`.
When a dynamic table-size update appears after a decoded header field in the
same completed header block, the HPACK fixture boundary uses id
`hpack.fixture.table_size_update_not_at_start` and also records
`observed_header_table_size`, `frame_kind`, `stream_id`, `stream_ref`, and
`active_state` before the same expected fixture, codec module, and byte
preview fields. Source-visible payloads for this id carry those fields in
`RuntimeHpackFixtureTableSizeUpdateDiagnostic(...)`.
Outbound header-list encode failures in the aggregate HTTP/2 run case stay as
typed HPACK fixture results in program stdout; they are not converted into
`details.protocol_diagnostic`.

Other non-zero Java process exits use `error.kind: "runtime"` with
`details.phase: "runtime"`. JDK setup failures use `error.kind: "runner"` with
`details.phase: "tool"`.
