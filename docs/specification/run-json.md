---
role: specification
authority: normative
update-when: The `veln run --json` output schema, runtime failure details, result failure projection, output events, summary shape, or executable run JSON evidence changes.
---

# Run JSON

`veln run --json` emits schema version `veln-run-json/v0` with:

- `command`: `run`
- `status`: `passed`, `failed`, or `error`
- `exit_code`: the captured Java process exit code, or `1` for tool errors
- `stdout`: captured user-program stdout
- `stderr`: captured user-program stderr
- `error`: `null` for passed runs, or a structured result, runtime, or runner
  error

If analysis reports parse, source, semantic, lowering, or run-entry effect
diagnostics before the backend starts, `veln run --json` emits the shared
diagnostic envelope used by diagnostic commands. The envelope uses
`schema_version: 1`, `status: "error"`, `diagnostics`, and `summary`, and stderr
is empty. The identifier-casing `*-json` run cases under
`examples/specification/run/` check this pre-execution diagnostic boundary for
reachable source declarations, aliases, type and constructor references,
handler bindings, and import recovery isolation.

Runtime contract failures use `error.kind: "contract"`. The error details use
`kind: "contract"` and `phase: "runtime"` and include:

- `clause`: `require`, `ensure`, or `invariant`
- `predicate`: the failed clause text
- `function`: the checked function boundary
- `blame`: `caller` for `require`, `implementation` for `ensure`, and either
  value for `invariant` depending on entry or return failure
- `node_id`: the contract node identifier
- `span`: the source span for the failed clause

Host runtime failures use `error.kind: "runtime"` and `details.phase:
"runtime"`. Structured transport failures additionally use
`details.id: "runtime.transport_failure"`; their primary `error.message`
contains only the failed operation and stable category. Their details project
`operation`, `category`, `lifecycle_phase`, known `local_endpoint` and
`peer_endpoint`, known `listener_id` or `stream_id`, known
`input_committed`, `output_committed`, and `ownership_committed` facts, and
related `platform_cause`. Unknown facts are omitted rather than inferred.
The focused `transport-socket-*-record-failure-json` cases under
`examples/specification/run/` check known endpoints and identities together
with ownership, input, or output commits after the corresponding host
transition. `transport-socket-write-record-failure-human` checks that the same
facts remain related notes in human output rather than entering the primary
message.
The `transport-socket-external-connect-failure-json` and
`transport-socket-external-listen-failure-json` cases apply the same payload
to host connection and bind failures in external runtime mode. Missing stream
or listener identities and false ownership commit facts demonstrate that the
failed operation did not create an in-memory fallback handle.
An invalid dynamic integer shift count additionally uses
`details.id: "runtime.invalid_shift_count"` and exposes `operator`,
`actual_count`, `minimum_count`, and `maximum_count`.
Non-socket descriptor failures, address metadata lookup failures, and forced
timeout or deadline expiry retain the generic host runtime shape.

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
Result-value containment assertions in executable run JSON payload cases are
harness evidence over these existing rendered string fields and do not add a
separate `veln run --json` field shape.

When the returned error value is
`RuntimeDiagnostic(id, message, RuntimeValueDiagnostic(...))` for a generated
binary schema encode failure id such as
`schema.encode_value_unrepresentable`, `details.value` keeps the rendered
`RuntimeDiagnostic(...)` value and `details.value_diagnostic` is projected
from that value. The value detail constructor carries the schema-local field
path segment list and reason text; run JSON derives `field_path`,
`field_path_display`, and `reason` from those fields while keeping the public
`value_diagnostic` shape used by generated `EncodeError(...)` result values.

When the returned error value is
`RuntimeDiagnostic(id, message, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDiagnostic(...)))`,
`details.value` likewise keeps the rendered `RuntimeDiagnostic(...)` value and
therefore exposes the nested envelope. The HPACK fixture detail still projects
to the unchanged `details.protocol_diagnostic` shape. The
unsupported-header-block, malformed-string-length, malformed-raw-string,
malformed-Huffman-padding, Huffman-EOS, and Huffman non-visible fixture
payloads, plus the source-visible HPACK static decoder
`hpack.static.unsupported_index` payload and malformed table-size update
integer payloads, carry byte offset, observed header block size, observed first
byte,
expected fixture, codec module, and a bounded header-block byte preview from
the returned error value itself. Dynamic-index fixture payloads use
`RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDynamicIndexDiagnostic(...))` to add
`requested_dynamic_index` and `dynamic_table_entry_count`. Table-size update
placement and trailing-byte payloads use
`RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureTableSizeUpdateDiagnostic(...))` to add
`observed_header_table_size`, `frame_kind`, `stream_id`, `stream_ref`, and
`active_state`.
Dynamic-name continuation payloads use
`RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDynamicNameDiagnostic(...))` to add
`requested_dynamic_index` and `dynamic_table_entry_count` for the focused
missing, malformed, and out-of-range continuation ids.
The standard `http2::hpack::diagnostic::unsupported_header_block(...)`,
`http2::hpack::diagnostic::malformed_string_length(...)`,
`http2::hpack::diagnostic::malformed_raw_string_value(...)`,
`http2::hpack::diagnostic::malformed_huffman_padding(...)`,
`http2::hpack::diagnostic::huffman_eos_symbol(...)`,
`http2::hpack::diagnostic::huffman_non_visible_value(...)`,
`http2::hpack::diagnostic::dynamic_index_out_of_range(...)`,
`http2::hpack::diagnostic::dynamic_name_continuation_missing(...)`,
`http2::hpack::diagnostic::dynamic_name_continuation_malformed(...)`,
`http2::hpack::diagnostic::dynamic_name_continuation_out_of_range(...)`,
`http2::hpack::diagnostic::table_size_update_malformed(...)`, and
`http2::hpack::diagnostic::table_size_update_not_at_start(...)`, and
`http2::hpack::diagnostic::table_size_update_trailing_bytes(...)` helpers return their
source-visible HPACK fixture `RuntimeDiagnostic(...)` payloads directly, so
their direct helper examples keep the rendered payload in `details.value` and
project the same HPACK fixture facts into `details.protocol_diagnostic`.

When the returned error value is an HTTP/2 protocol
`RuntimeDiagnostic(...)` payload, `details.value` keeps the rendered
`RuntimeDiagnostic(...)` value with the
`RuntimeHttp2Diagnostic(Http2DiagnosticDetail)` envelope, while the inner
detail projects to the unchanged `details.protocol_diagnostic` shape. The
implemented HTTP/2 payload constructors are
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolClosedWithPendingDiagnostic(...))` for
`http2.protocol.closed_with_pending`,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolPartialPrefaceDiagnostic(...))` for
`http2.protocol.partial_preface`,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidPrefaceDiagnostic(...))` for
`http2.protocol.invalid_preface`,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInitialPeerSettingsRequiredDiagnostic(...))` for
`http2.protocol.initial_peer_settings_required`,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolContinuationExpectedDiagnostic(...))` for
`http2.protocol.continuation_expected`,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic(...))` for
`http2.protocol.unexpected_settings_ack`,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidFrameKindDiagnostic(...))` for
`http2.protocol.invalid_frame_kind`,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidStreamIdDiagnostic(...))` for
`http2.protocol.invalid_stream_id`,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolPeerStreamIdNotIncreasingDiagnostic(...))` for
`http2.protocol.peer_stream_id_not_increasing`,
`RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitFrameSizeDiagnostic(...))` for
`http2.peer_limit.frame_size_exceeded`,
`RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitHeaderListSizeDiagnostic(...))` for
`http2.peer_limit.header_list_size_exceeded`,
`RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic(...))` for
`http2.peer_limit.header_table_size_exceeded`,
`RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic(...))` for
`http2.peer_limit.concurrent_streams_exceeded`,
`RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitSettingsValueDiagnostic(...))` for
`http2.peer_limit.settings_value_out_of_range`,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic(...))` for
`http2.protocol.invalid_payload_length`,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic(...))` for
`http2.protocol.invalid_data_padding`,
`RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitFlowControlWindowDiagnostic(...))` for
`http2.peer_limit.flow_control_window_exceeded`,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolContentLengthMismatchDiagnostic(...))` for
`http2.protocol.content_length_mismatch`,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic(...))` for
`http2.protocol.invalid_request_header_list`,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic(...))` for
`http2.protocol.invalid_response_header_list`,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic(...))` for
`http2.protocol.invalid_window_update_increment`,
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolPriorityDependencyDiagnostic(...))` for
`http2.protocol.invalid_priority_dependency`, and
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic(...))` for
`http2.protocol.stream_after_goaway`. These constructors project the same
public JSON fields as the compatibility helpers, including stream
classification, peer-limit facts, active state, rule provenance, receive-limit
provenance, header-list facts, decoded header names, and bounded byte previews
where applicable.
The `http2::diagnostic::protocol_closed_with_pending(...)`,
`http2::diagnostic::protocol_partial_preface(...)`,
`http2::diagnostic::protocol_invalid_preface(...)`,
`http2::diagnostic::protocol_initial_peer_settings_required(...)`,
`http2::diagnostic::protocol_continuation_expected(...)`,
`http2::diagnostic::peer_limit_frame_size_exceeded(...)`,
`http2::diagnostic::peer_limit_header_list_size_exceeded(...)`,
`http2::diagnostic::peer_limit_header_table_size_exceeded(...)`,
`http2::diagnostic::peer_limit_concurrent_streams_exceeded(...)`, and
`http2::diagnostic::peer_limit_settings_value_out_of_range(...)` helpers return their
source-visible HTTP/2 `RuntimeDiagnostic(...)` payloads directly, so
`details.value` is the rendered payload instead of a plain string.
The `http2::diagnostic::protocol_invalid_window_update_increment(...)`,
`http2::diagnostic::protocol_content_length_mismatch(...)`,
`http2::diagnostic::protocol_invalid_priority_dependency(...)`,
`http2::diagnostic::protocol_stream_after_goaway(...)`, and
`http2::diagnostic::peer_limit_flow_control_window_exceeded(...)` standard helpers also
return their source-visible HTTP/2 `RuntimeDiagnostic(...)` payloads directly;
their direct helper examples keep the rendered payload in `details.value` and
project the same protocol facts into `details.protocol_diagnostic`.
The `http2::diagnostic::protocol_invalid_stream_id(...)` standard helper likewise returns
the source-visible HTTP/2 `RuntimeDiagnostic(...)` payload directly; its direct
helper example keeps the rendered payload in `details.value` and projects the
same stream id domain facts into `details.protocol_diagnostic`.
The `http2::diagnostic::protocol_invalid_data_padding(...)` and
`http2::diagnostic::protocol_unexpected_settings_ack(...)` standard helpers likewise return
source-visible HTTP/2 `RuntimeDiagnostic(...)` payloads directly; their direct
helper examples keep the rendered payload in `details.value` and project the
same DATA padding or SETTINGS ACK facts into `details.protocol_diagnostic`.
The `http2::diagnostic::protocol_invalid_request_header_list(...)` and
`http2::diagnostic::protocol_invalid_response_header_list(...)` standard helpers likewise
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
Anonymous record field failures append anonymous record field segments after
the outer field segment without adding a synthetic schema segment. The checked
JSON cases are
`examples/specification/run/binary-schema-anonymous-record-truncated-json/`,
`examples/specification/run/binary-schema-nested-anonymous-record-truncated-json/`,
`examples/specification/run/binary-schema-sibling-nested-anonymous-record-truncated-json/`,
and
`examples/specification/run/binary-schema-recursive-anonymous-record-truncated-json/`.
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

When the result value is a binary schema byte-view payload multiple failure,
`details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "schema.length_multiple_mismatch"`
- `byte_offset`: the decoded-stream `ByteOffset` where the payload starts
- `field_path`: schema-local path segment objects with `kind` and `name`
- `observed_count`: the computed payload byte count
- `required_multiple`: the required multiple value
- `multiple_operand`: the earlier field name or positive integer literal
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
- `actual_value`: the decoded integer value that was present when it fits a
  signed JSON number
- `actual_value_text`: the decoded integer value as decimal text when the
  value is too large for a signed JSON number; this replaces `actual_value`
  for that diagnostic instance
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
representation-only reserved field. The checked
`examples/specification/run/binary-schema-general-reserved-byte-prefix-json/`
case covers a nonzero three-bit prefix before a visible byte under the general
padded prefix rule.

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
`EncodeError(id, field_path, reason)` with a supported encode diagnostic id,
or a `veln run` entry returns
`EncodeStep::Invalid(EncodeError(id, field_path, reason))`, or the entry
returns `Err(RuntimeDiagnostic(id, message, RuntimeValueDiagnostic(field_path,
reason)))` for the same supported encode ids,
`details.value_diagnostic` includes:

- `kind: "value_diagnostic"`
- `id`: one of `schema.encode_value_unrepresentable`,
  `schema.dispatch_unknown_tag`, `schema.dispatch_length_mismatch`,
  `schema.dispatch_mismatch`, or `schema.validation_failed`. Hand-written
  codec encode values may still use `codec.encode_value_unrepresentable` or
  compatibility-only `codec.dispatch_*` ids.
- `field_path`: schema-local path segment objects with `kind` and `name`,
  derived from the source-visible field path
- `field_path_display`: the source-visible field path string for
  representation and dispatch failures
- `reason`: the source-visible encode failure reason for representation and
  dispatch failures
- `expected_count`, `actual_count`, `length_expression`, `byte_offset`, and
  `byte_preview` for generated length-bounded `ByteView` encode count
  mismatches
- `predicate`, `field_value`, `supplied_values`, and supplied schema-local
  `Int` values keyed by field name for encode-time `schema.validation_failed`

`EncodeStep::Encoded(...)` and `EncodeStep::Partial(...)` entry results do not
populate `error` or `details.value_diagnostic`.

Generated binary schema encode failures inside anonymous record fields append
the anonymous record field segments after the outer field segment without a
synthetic schema segment. The checked JSON case is
`examples/specification/run/binary-schema-anonymous-record-encode-out-of-range-json/`.

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
- `expected_checksum`: the expected checksum string when the id is
  `codec.checksum_mismatch` and the source-visible reason carries checksum
  mismatch fields
- `actual_checksum`: the actual checksum string when the id is
  `codec.checksum_mismatch` and the source-visible reason carries checksum
  mismatch fields
- `expected_length`: the expected length number when the id is
  `codec.length_mismatch` and the source-visible reason carries length
  mismatch fields
- `actual_length`: the actual length number when the id is
  `codec.length_mismatch` and the source-visible reason carries length
  mismatch fields
- `expected_payload_length`: the expected payload length number when the id
  is `codec.payload_length_mismatch` and the source-visible reason carries
  payload length mismatch fields
- `actual_payload_length`: the actual payload length number when the id is
  `codec.payload_length_mismatch` and the source-visible reason carries
  payload length mismatch fields
- `expected_padding_length`: the expected padding length number when the id is
  `codec.padding_mismatch` and the source-visible reason carries padding
  mismatch fields
- `actual_padding_length`: the actual padding length number when the id is
  `codec.padding_mismatch` and the source-visible reason carries padding
  mismatch fields
- `byte_width`: the integer byte width number when the id is
  `codec.integer_out_of_range` and the source-visible reason carries integer
  range fields
- `min_value`: the accepted minimum integer value when the id is
  `codec.integer_out_of_range` and the source-visible reason carries integer
  range fields
- `max_value`: the accepted maximum integer value when the id is
  `codec.integer_out_of_range` and the source-visible reason carries integer
  range fields
- `actual_value`: the decoded integer value when the id is
  `codec.integer_out_of_range` and the source-visible reason carries integer
  range fields
- `expected_sequence`: the expected sequence string when the id is
  `codec.sequence_mismatch` and the source-visible reason carries sequence
  mismatch fields
- `actual_sequence`: the actual sequence string when the id is
  `codec.sequence_mismatch` and the source-visible reason carries sequence
  mismatch fields
- `expected_version`: the expected version string when the id is
  `codec.version_mismatch` and the source-visible reason carries version
  mismatch fields
- `actual_version`: the actual version string when the id is
  `codec.version_mismatch` and the source-visible reason carries version
  mismatch fields
- `expected_tag`: the expected tag string when the id is
  `codec.tag_mismatch` and the source-visible reason carries tag mismatch
  fields
- `actual_tag`: the actual tag string when the id is
  `codec.tag_mismatch` and the source-visible reason carries tag mismatch
  fields
- `expected_magic`: the expected magic string when the id is
  `codec.magic_mismatch` and the source-visible reason carries magic
  mismatch fields
- `actual_magic`: the actual magic string when the id is
  `codec.magic_mismatch` and the source-visible reason carries magic
  mismatch fields
- `unsupported_feature`: the unsupported feature string when the id is
  `codec.unsupported_feature` and the source-visible reason carries the
  unsupported feature field
- `consumed_count`: the logical value's consumed byte count when the id is
  `codec.trailing_input` and the source-visible reason carries consistent
  trailing-input counts
- `remaining_count`: the positive byte count left after the logical value when
  the id is `codec.trailing_input` and the source-visible reason carries
  consistent trailing-input counts
- `local_byte_offset`: the byte offset reported by helper context carried by
  the reason when present
- `expected_count`: the byte count expected by helper context carried by the
  reason when present
- `available_count`: the byte count available to helper context carried by the
  reason when present
- `byte_preview`: a structured bounded byte preview object for helper context
  carried by the reason when present

This shape also covers codec-owned invalid-input facts returned by an
ordinary decode function, such as `codec.invalid_input` and
`codec.packet_kind_invalid`, and direct `Result<_, DecodeError>` failures
that carry those codec-owned ids. A plain `DecodeErrorWithReason` reason is
kept only as `reason`; helper-only fields are omitted unless the reason
matches registered helper context. If the reason is a byte-helper failure
message with registered helper context, the command-facing projection keeps
the reason text and adds the carried helper counts, local byte offset, and
byte preview to the same `details.byte_diagnostic`. The checked
packet-kind examples cover direct `DecodeErrorWithReason(...)` result
failures and `Invalid(DecodeErrorWithReason(...))` entry results in
`examples/specification/run/codec-packet-kind-invalid-direct-json/` and
`examples/specification/run/codec-packet-kind-invalid-step-json/`.
For `codec.checksum_mismatch`, a source-visible reason written as
`expected_checksum=<value>; actual_checksum=<value>; reason=<text>` is
projected as separate `expected_checksum`, `actual_checksum`, and `reason`
fields. The checked direct result and `DecodeStep::Invalid(...)` examples are
`examples/specification/run/codec-checksum-mismatch-direct-json/` and
`examples/specification/run/codec-checksum-mismatch-step-json/`.
For `codec.length_mismatch`, a source-visible reason written as
`expected_length=<n>; actual_length=<n>; reason=<text>` is projected as
separate numeric `expected_length`, numeric `actual_length`, and `reason`
fields. Plain reason strings still keep only `reason` and do not invent
length facts. The checked direct result and `DecodeStep::Invalid(...)`
examples are
`examples/specification/run/codec-length-mismatch-direct-json/` and
`examples/specification/run/codec-length-mismatch-step-json/`.
For `codec.payload_length_mismatch`, a source-visible reason written as
`expected_payload_length=<n>; actual_payload_length=<n>; reason=<text>` is
projected as separate numeric `expected_payload_length`, numeric
`actual_payload_length`, and `reason` fields. Plain reason strings still keep
only `reason` and do not invent payload length facts. The checked direct
result and `DecodeStep::Invalid(...)` examples are
`examples/specification/run/codec-payload-length-mismatch-direct-json/` and
`examples/specification/run/codec-payload-length-mismatch-step-json/`.
For `codec.padding_mismatch`, a source-visible reason written as
`expected_padding_length=<n>; actual_padding_length=<n>; reason=<text>` is
projected as separate numeric `expected_padding_length`, numeric
`actual_padding_length`, and `reason` fields. Plain reason strings still keep
only `reason` and do not invent padding facts. The checked direct result and
`DecodeStep::Invalid(...)` examples are
`examples/specification/run/codec-padding-mismatch-direct-json/` and
`examples/specification/run/codec-padding-mismatch-step-json/`.
For `codec.integer_out_of_range`, a source-visible reason written as
`byte_width=<n>; min_value=<n>; max_value=<n>; actual_value=<n>; reason=<text>`
is projected as separate numeric `byte_width`, `min_value`, `max_value`,
`actual_value`, and `reason` fields. Plain reason strings still keep only
`reason` and do not invent integer range facts. The checked direct result and
`DecodeStep::Invalid(...)` examples are
`examples/specification/run/codec-integer-out-of-range-direct-json/` and
`examples/specification/run/codec-integer-out-of-range-step-json/`.
For `codec.sequence_mismatch`, a source-visible reason written as
`expected_sequence=<value>; actual_sequence=<value>; reason=<text>` is
projected as separate `expected_sequence`, `actual_sequence`, and `reason`
fields. Plain reason strings still keep only `reason` and do not invent
sequence facts. The checked direct result and `DecodeStep::Invalid(...)`
examples are
`examples/specification/run/codec-sequence-mismatch-direct-json/` and
`examples/specification/run/codec-sequence-mismatch-step-json/`.
For `codec.version_mismatch`, a source-visible reason written as
`expected_version=<value>; actual_version=<value>; reason=<text>` is
projected as separate `expected_version`, `actual_version`, and `reason`
fields. Plain reason strings still keep only `reason` and do not invent
version facts. The checked direct result and `DecodeStep::Invalid(...)`
examples are
`examples/specification/run/codec-version-mismatch-direct-json/` and
`examples/specification/run/codec-version-mismatch-step-json/`.
For `codec.tag_mismatch`, a source-visible reason written as
`expected_tag=<value>; actual_tag=<value>; reason=<text>` is projected as
separate `expected_tag`, `actual_tag`, and `reason` fields. Plain reason
strings still keep only `reason` and do not invent tag facts. The checked
direct result and `DecodeStep::Invalid(...)` examples are
`examples/specification/run/codec-tag-mismatch-direct-json/` and
`examples/specification/run/codec-tag-mismatch-step-json/`.
For `codec.magic_mismatch`, a source-visible reason written as
`expected_magic=<value>; actual_magic=<value>; reason=<text>` is projected as
separate `expected_magic`, `actual_magic`, and `reason` fields. Plain reason
strings still keep only `reason` and do not invent magic facts. The checked
direct result and `DecodeStep::Invalid(...)` examples are
`examples/specification/run/codec-magic-mismatch-direct-json/` and
`examples/specification/run/codec-magic-mismatch-step-json/`.
For `codec.unsupported_feature`, a source-visible reason written as
`feature=<value>; reason=<text>` is projected as separate
`unsupported_feature` and `reason` fields. Plain reason strings still keep
only `reason` and do not invent feature facts. The checked direct result and
`DecodeStep::Invalid(...)` examples are
`examples/specification/run/codec-unsupported-feature-direct-json/` and
`examples/specification/run/codec-unsupported-feature-step-json/`.
For `codec.trailing_input`, a source-visible reason written as
`consumed_count=<n>; available_count=<n>; remaining_count=<n>; reason=<text>`
is projected as separate numeric `consumed_count`, `available_count`,
`remaining_count`, and `reason` fields when the counts are nonnegative,
remaining is positive, and consumed plus remaining equals available. Plain or
malformed reason shapes keep only the original `reason` and do not invent
count facts. The checked direct result and `DecodeStep::Invalid(...)` examples
are `examples/specification/run/codec-trailing-input-direct-json/` and
`examples/specification/run/codec-trailing-input-step-json/`; malformed
fallback behavior is checked by
`examples/specification/run/codec-trailing-input-malformed-direct-json/`, and
plain-reason fallback behavior is checked by
`examples/specification/run/codec-trailing-input-plain-step-json/`.

The checked `codec.consumed_count_invalid` command-facing slice covers
hand-written decode boundaries whose returned `Decoded` consumed count is
outside the supplied `ByteView`. It uses the same `DecodeStep::Invalid(...)`
JSON shape and does not set `readiness`, because the supplied bytes already
prove the consumed-count fact false. When the source-visible reason is written
as `available_count=<count>; actual_consumed_count=<count>; reason=<text>`,
`details.byte_diagnostic` includes separate `available_count`,
`actual_consumed_count`, and `reason` fields. JVM runtime tests cover boundary
validation for oversized and negative counts; the checked command-facing JSON
examples are `examples/specification/run/codec-consumed-count-invalid-json/`
and `examples/specification/run/codec-consumed-count-invalid-negative-json/`.

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
program stdout in focused outbound HTTP/2 cases; they do not populate `error` or
`details.protocol_diagnostic`. The HTTP/2 adapter/core write boundary likewise
prints its adapter summary as ordinary stdout, records accepted HEADERS and
split DATA chunks in the fixture transport log, and leaves rejected send
actions out of `error` unless adapter code explicitly reports them. The same
applies when those send-intents build
their opaque header-block bytes from header-list values through the
production HPACK encoder, including exact static-indexed HPACK bytes for
fixed-value HPACK static table entries such as request pseudo-headers,
response pseudo-headers, and ordinary headers on outbound HEADERS, exact
static-indexed bytes for supported fixed-value static entries on
`PUSH_PROMISE`, checked Huffman-marked string literal fixtures for outbound
HEADERS and `PUSH_PROMISE`, and the checked stateful `PUSH_PROMISE` path
where the returned encode state lets a later promised header list use
the dynamic indexed byte `0xbe`. The focused HPACK fixture stdout also checks
static-name literal-without-indexing encoding through finite static-table
name metadata: non-exact `:method: PUT`, ordinary `server: ok`, and the
existing `:path: /target` subset emit raw literal bytes, while a non-static
name remains a fixture encode failure. The same run JSON stdout checks the
source-visible HPACK Huffman payload helper directly: successful string and
bounded byte inputs print payload-only output chunks without the HPACK string
length prefix, and unsupported source strings remain ordinary fixture
`Result` failures printed as stdout. The outbound
HEADERS fixture path also checks a raw ordinary new-name
literal-never-indexed header block: it emits deterministic HPACK bytes
without inserting `x-never: no` into the dynamic table, a later dynamic-index
probe for that field fails at the fixture boundary, and the earlier
`:path: /target` dynamic entry remains reusable as `0xbe`. The same checked
HEADERS and `PUSH_PROMISE` state handoff covers a fixture-scoped
dynamic-name literal-with-indexing encode: a fresh `:path: /again` value is
inserted as the newest dynamic entry, later reused as `0xbe`, and the older
`:path: /target` entry remains reachable as `0xbf`. The HEADERS path also
checks a dynamic-name literal-never-indexed encode for `:path: /secret`: it
emits the never-indexed dynamic-name bytes, does not insert `/secret`, and
keeps the earlier `:path: /target` entry reusable as `0xbe`. Outbound HPACK
dynamic-name Huffman-value stdout covers literal-without-indexing,
literal-with-indexing, and literal-never-indexed. The indexed form inserts
`:path: test` for later `0xbe` reuse, while the other forms retain the older
`:path: /target` entry. The same run routes all three encoded blocks through
outbound HEADERS and keeps missing dynamic-name state and unsupported Huffman
input on no-output fixture failure paths. Outbound HPACK
representation-selector stdout records the exact seed block containing static
indexed `:method: GET` and a new-name `x-trace: ok` insertion. A second block
records exact dynamic reuse followed by dynamic-name insertion of
`x-trace: again`. The focused boundary also covers static-name precedence,
reduced-capacity eviction, invalid-name failure, capacity mismatch, and reuse
of the original carried state after failures. The aggregate case tries the
selector before the existing fixture encoder fallback, routes both mixed
blocks through outbound HEADERS, and records their exact frame bytes.
Outbound HPACK
dynamic table-size update requests use the same result boundary: an accepted
update returns a fixture encode state that later HEADERS and `PUSH_PROMISE`
encodes consume before frame splitting, while an over-limit update remains a
typed HPACK fixture encode failure and produces no HTTP/2 output chunk list.
The aggregate HEADERS path also checks zero-capacity and reduced-capacity
insertion: after a table-size update to zero, the returned fixture state has no
dynamic entries and repeated literal-with-indexing `:method: PUT` HEADERS
encodes keep emitting the literal bytes instead of dynamic indexed reuse. After
a table-size update to `30`, the same block is also emitted as a literal again
on the next encode because it does not fit the current fixture table, while a
table-size update to `42` lets the same entry fit and be reused as `0xbe` on
the following HEADERS encode.
Received peer `SETTINGS_HEADER_TABLE_SIZE` values provide the outbound HPACK
fixture capacity for those update requests without changing the local inbound
HPACK table-size receive limit.

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
value with `RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolPartialPrefaceDiagnostic(...))` or
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidPrefaceDiagnostic(...))`. Input
end with pending bytes after the preface uses id
`http2.protocol.closed_with_pending` and records `byte_offset.value`,
`pending_count`, `input_event`, `active_continuation`,
`expected_stream_id`, `started_frame_kind`, `started_byte_offset`,
`accumulated_header_block_bytes`, and `rule_provenance`, plus a structured
bounded `byte_preview` for the retained pending bytes. A frame that violates
an active header-block continuation sequence uses id
`http2.protocol.continuation_expected` and records `byte_offset.value`,
`actual_frame_kind`, `actual_stream_id`, `expected_stream_id`,
`started_frame_kind`, `started_byte_offset`, `active_continuation`,
`accumulated_header_block_bytes`, and `rule_provenance`, plus a structured
bounded `byte_preview` for the inspected incoming frame header bytes. These
protocol-owned byte previews use the same `encoding`, `data`,
`preview_byte_count`, `total_byte_count`, and `truncated` object shape as
schema-owned byte diagnostics, while byte offset, expected byte count, actual
pending count, matched prefix count, expected byte, actual byte, active
protocol state, active continuation state, accumulated header-block byte
count, and rule provenance stay in their own fields. The checked closed-input
pending-byte and continuation-ordering JSON examples return source-visible
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolClosedWithPendingDiagnostic(...))` and
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolContinuationExpectedDiagnostic(...))` payloads, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields. The
`http2::diagnostic::protocol_closed_with_pending`,
`http2::diagnostic::protocol_continuation_expected`,
`http2::diagnostic::peer_limit_frame_size_exceeded`,
`http2::diagnostic::peer_limit_header_table_size_exceeded`, and
`http2::diagnostic::peer_limit_concurrent_streams_exceeded` standard helpers return the
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
DATA receive-window, or HPACK table-size receive-limit failures. A received
`SETTINGS_INITIAL_WINDOW_SIZE` delta can still change the tracked open
stream's allowed receive-window credit, and later DATA failures report that
adjusted credit.
Unknown SETTINGS identifiers do not update peer-advertised state and do not produce
`http2.peer_limit.settings_value_out_of_range`; known SETTINGS items in the
same frame are still applied or diagnosed at their own item byte offset.
Duplicate known SETTINGS identifiers are processed in wire order and the last
occurrence supplies the active peer-advertised value. For repeated
`SETTINGS_INITIAL_WINDOW_SIZE`, every ordered delta is applied to all tracked
open outbound streams without changing connection credit, body accounting, or
closed and reset lifecycle. Validation precedes the whole frame update, so an
invalid later duplicate keeps the existing peer state and outbound credit
unchanged while its focused human and JSON projections retain that duplicate
item's byte offset and six-byte preview. The focused projections are checked
by `examples/specification/run/http2-protocol-core-settings-value-json/` and
`examples/specification/run/http2-protocol-core-settings-value-human/`;
broader HTTP/2 receive behavior is routed from `http2.md`.
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
`RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitFlowControlWindowDiagnostic(...))` payloads, so
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
application-byte preview. The checked run examples cover both over-length
outbound DATA and an early local `END_STREAM` shortfall through source-visible
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolContentLengthMismatchDiagnostic(...))` payloads. A
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
The `http2::diagnostic::peer_limit_header_table_size_exceeded(...)` standard helper
returns the same `RuntimeDiagnostic(...)` value directly, so its JSON result
details are also derived from the returned value.
The `http2::diagnostic::peer_limit_concurrent_streams_exceeded(...)` standard helper
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
request, empty `:method`, invalid `:scheme`, empty `:path`, invalid
`:authority`, ordinary `CONNECT` with missing or empty `:authority`, ordinary
`CONNECT` with forbidden `:scheme` or `:path`, and invalid and mismatched
`content-length` values; the larger
protocol-core fixture also checks the integrated completed HEADERS and final
CONTINUATION paths, including accepted `:scheme` values `http` and `https`,
accepted `te: trailers`, accepted `content-length` values, and accepted
ordinary `CONNECT` with a non-empty `:authority` and no `:scheme` or `:path`.
The ordinary `CONNECT` failures use stable facts
`connect_authority_missing`, `connect_authority_empty`,
`connect_scheme_present`, and `connect_path_present`, with header names,
decoded names, stream context, and
`rfc9113_connect_request_pseudo_headers` provenance kept in structured
protocol-diagnostic details. The aggregate
protocol-core run case also checks source-visible HPACK static-name
`:scheme` literals after a static request `:method` and before a static
request `:path`: decoded values `http` and `https` are accepted, while other
visible ASCII values use the existing
`scheme_value_not_http_or_https` request header-list fact across completed
HEADERS and final CONTINUATION paths. The same aggregate case checks
source-visible HPACK static-name `:authority` literals after static request
`:method` and `:scheme` pseudo-headers and before a static request `:path`;
accepted visible ASCII values pass, while the checked invalid visible ASCII
value uses the existing `authority_value_invalid` request header-list fact on
completed HEADERS and final CONTINUATION paths. It also checks
source-visible HPACK static-name `content-length` literals after static
request pseudo-headers and after a static response `:status` pseudo-header
across the
literal-without-indexing, literal-with-indexing, and literal-never-indexed
forms that do not require later fixture dynamic-table reuse; the decoded
values feed the same matching request or response header-list validation and
content-length body accounting paths, including rejection of non-decimal
values. The focused request
header-list JSON examples,
including the raw HPACK uppercase and invalid-token trailer-name projections,
return
source-visible `RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic(...))`
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
output and invalid and mismatched `content-length` values. The larger
protocol-core fixture also checks `:authority` as request-only, and rejects
empty, short, long, and non-decimal `:status` values with
`status_value_invalid` after fixture decode and source-visible HPACK
static-name literal decode on completed HEADERS and final CONTINUATION paths.
The focused response
header-list human and JSON examples return source-visible
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic(...))` payloads, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields, including the
header-block preview. The larger
protocol-core fixture also checks valid ordinary response header lists,
including accepted `te: trailers` and accepted `content-length` values,
through integrated completed HEADERS and final CONTINUATION paths.
Final `204` and `304` response HEADERS retain a no-content stream state when
they omit `END_STREAM`. Empty DATA and PADDED DATA with zero application
content may terminate that state, while nonempty DATA uses
`http2.protocol.content_length_mismatch` with
`expected_content_length: 0`, the observed application length, status-bearing
`active_state`, and `rfc9110_no_content_response_body` provenance. Focused
content-length and no-content cases check direct and CONTINUATION transitions
plus diagnostic projection; the focused human case checks the status-specific
primary message and related state and provenance notes. Focused
response-trailer cases check validation through the same response header-list
diagnostic fields with active state `response-trailers`; a focused JSON case
checks the same active state and the inspected header-block `byte_preview` in
diagnostic projection.
Received
SETTINGS range failures use id
`http2.peer_limit.settings_value_out_of_range` and record
`byte_offset.value`, `setting_identifier`, `setting_name`, `observed_value`,
`accepted_min_value`, `accepted_max_value`, `peer_limit_provenance`, and a
structured bounded `byte_preview` for the offending six-byte SETTINGS item.
The preview uses the same object shape as other protocol-owned byte previews
while byte offset, setting identity, observed value, accepted range, and
peer-limit provenance remain separate fields. The
`http2::diagnostic::peer_limit_settings_value_out_of_range(...)` standard helper returns
the same `RuntimeDiagnostic(...)` value directly, so its JSON result details
are also derived from the returned value. The
stream id domain slice uses id `http2.protocol.invalid_stream_id` and records
`byte_offset.value`, `frame_kind`, `stream_id`, `stream_ref`,
`required_stream_id_domain`, `endpoint_role`, `active_state`, and
`rule_provenance`, plus a structured bounded `byte_preview` for the inspected
frame-header bytes. Source-visible
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidStreamIdDiagnostic(...))` payloads keep the
rendered `RuntimeDiagnostic(...)` in `details.value` while projecting the same
protocol diagnostic fields. The standard
`http2::diagnostic::protocol_invalid_stream_id(...)` helper returns the same
`RuntimeDiagnostic(...)` value directly, so its JSON result details are also
derived from the returned value.
The preview uses the same object shape as other protocol-owned byte previews
while stream id domain facts stay in their own fields; the checked HTTP/2
examples cover invalid zero stream ids, even client stream ids, nonzero stream
ids on connection-only frames, and CONTINUATION on the connection stream while
a nonzero-stream header block is pending. The
peer-created stream ordering slice uses id
`http2.protocol.peer_stream_id_not_increasing` and records
`byte_offset.value`, `stream_id`, `stream_ref`,
`previous_peer_stream_id`, `endpoint_role`, `active_state`, and
`rule_provenance`, plus a structured bounded `byte_preview` of the attempted
HEADERS or `PUSH_PROMISE` frame header. Source-visible
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolPeerStreamIdNotIncreasingDiagnostic(...))` payloads keep
the rendered `RuntimeDiagnostic(...)` in `details.value` while projecting the
same fields. Focused human and JSON examples are under
`examples/specification/run/http2-protocol-core-peer-stream-id-monotonicity-human/`
and
`examples/specification/run/http2-protocol-core-peer-stream-id-monotonicity-json/`.
The client promised-stream ordering projection reuses the same payload and
fields with endpoint role `client`; focused human and JSON examples are under
`examples/specification/run/http2-protocol-core-client-promised-stream-id-ordering-human/`
and
`examples/specification/run/http2-protocol-core-client-promised-stream-id-ordering-json/`.
The
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
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidFrameKindDiagnostic(...))` payloads, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields.
The broad HTTP/2 protocol-core run example also fixes ordinary stdout evidence
for client-side peer-sent `PUSH_PROMISE` receive: accepted single-frame and
final-CONTINUATION cases expose the stripped promised header block as checked
lowercase hex output and print the reserved-by-peer promised stream state.
The same executable case checks PADDED single-frame and final-CONTINUATION
receive without adding command output: only the unpadded header block reaches
HPACK, zero padding is accepted, and truncated prefixes or excessive padding
fail before promised-stream and connection-state updates. The focused
`examples/specification/run/http2-protocol-core-push-promise-padding-json/`
case fixes the excessive-padding count, frame kind, associated stream, rule
provenance, and inspected payload preview in `protocol_diagnostic` details.
The same checked stdout records the latest locally sent `SETTINGS_ENABLE_PUSH`
state after outbound settings send-intents, keeps accepted peer-sent
`PUSH_PROMISE` behavior unchanged when local push is enabled or unspecified,
and rejects a valid peer-sent `PUSH_PROMISE` before reservation when local
push is disabled. The rejected case uses
`http2.protocol.invalid_frame_kind` with active state `local-settings`, rule
provenance `local_settings_enable_push_disabled`, and prints that the
promised stream remains unreserved.
That checked stdout also records local SETTINGS batches as ordered
frame-header-plus-payload chunks. It covers a three-item batch whose emitted
identifier/value pairs remain in caller order, a peer SETTINGS ACK that clears
that multi-item batch while preserving a later outstanding batch. Local
`SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_MAX_CONCURRENT_STREAMS`, and
`SETTINGS_MAX_HEADER_LIST_SIZE` values must be representable in the HTTP/2
four-byte unsigned SETTINGS value field. Invalid items inside larger local
batches emit no output chunk, record no outstanding local SETTINGS batch, and
keep `local_settings` provenance on
`http2.peer_limit.settings_value_out_of_range`.
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
For outbound promised stream id reuse or regression,
`http2.protocol.peer_stream_id_not_increasing` carries the attempted promised
stream id as `stream_id`, the retained local high-water value as
`previous_peer_stream_id`, server `endpoint_role`, `active_state`,
`rule_provenance`, and the bounded preview fields. The focused JSON case is
`examples/specification/run/http2-protocol-core-outbound-promised-stream-id-ordering-json/`.
For outbound local HEADERS stream-id reuse or regression, the same diagnostic
payload carries the attempted client-initiated id as `stream_id`, the retained
local high-water value as `previous_peer_stream_id`, client `endpoint_role`,
`active_state`, `rule_provenance`, and the bounded preview fields. The focused
JSON case is
`examples/specification/run/http2-protocol-core-outbound-local-stream-id-ordering-json/`.
After receiving GOAWAY or after locally sending GOAWAY, a peer-created
HEADERS stream, local outbound HEADERS send-intent, local outbound
`PRIORITY` send-intent, stream-level outbound `WINDOW_UPDATE` receive-credit
intent, or server-side outbound `PUSH_PROMISE` send-intent greater than the
recorded last stream id uses id
`http2.protocol.stream_after_goaway` and records
`byte_offset.value`, `stream_id`, `stream_ref`, `last_stream_id`,
`shutdown_state`, `endpoint_role`, `byte_preview`, `active_state`, and
`rule_provenance`. `shutdown_state` is the active shutdown label, including
`drained_shutdown` after drain completion. The peer-created receive case
carries a bounded inspected frame-header preview; the local outbound helper
form can carry an empty preview when no peer bytes were inspected. Focused
outbound HTTP/2 cases check that above-boundary outbound `PUSH_PROMISE`
rejects before HPACK encoding, that above-boundary outbound `PRIORITY` rejects
before priority payload encoding or emitted bytes, and that above-boundary
outbound `WINDOW_UPDATE` rejects before receive-credit changes and emitted
bytes. Focused cases keep connection-level outbound `WINDOW_UPDATE` accepted
after GOAWAY and keep priority self-dependency on its narrower
diagnostic path.
Repeated local outbound GOAWAY send-intents in focused GOAWAY output cases
emit normal GOAWAY bytes when the new last-stream id preserves or narrows the
recorded local shutdown boundary. A repeated local GOAWAY that would widen
the recorded boundary uses `http2.protocol.stream_after_goaway` with local
endpoint context and emits no output chunk. Later local outbound stream
send-intents continue to use the narrowed local boundary.
Focused checked output preserves empty and non-empty inbound GOAWAY opaque
debug data as exact hexadecimal bytes from the ordinary receive result. Its
outbound chunk list checks that the same non-text byte sequence follows the
fixed last-stream-id and error-code fields and that the frame header carries
the complete payload length. Payloads shorter than eight bytes remain
`http2.protocol.invalid_payload_length` with the primary failed fact limited
to the observed and required payload lengths.
The checked `run --json` protocol-core example also keeps already-admitted
peer-created stream DATA and trailer HEADERS after received GOAWAY as passed
stdout, not as a `protocol_diagnostic`; receive-window credit, HPACK fixture
decode, and closed-by-peer lifecycle facts remain ordinary executable output.
The aggregate example checks these accepted frames for streams at the received
last-stream-id boundary and below it, and separately rejects a new
above-boundary peer-created HEADERS frame while the shutdown state is still
`graceful_shutdown`.
The checked stream-after-GOAWAY human and JSON examples return
source-visible `RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic(...))`
payloads for peer-created and local outbound stream send-intents, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields and structured
preview object.
Wrong-length protocol payloads use id
`http2.protocol.invalid_payload_length` and record `byte_offset.value`,
`frame_kind`, `stream_id`, `stream_ref`, `observed_payload_length`,
`expected_payload_length`, `active_state`, and `rule_provenance`, plus a
structured bounded `byte_preview` for the inspected payload bytes. The preview
uses the same object shape as other protocol-owned byte previews while
payload length facts stay in their own fields. The SETTINGS ACK, SETTINGS
item-width, PING, GOAWAY, `RST_STREAM`, and `WINDOW_UPDATE` checked JSON
examples return
source-visible
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic(...))` payloads, and the
`http2::diagnostic::protocol_invalid_payload_length(...)` helper returns the same
source-visible payload directly. The checked helper JSON example also covers
the `WINDOW_UPDATE` fixed payload-length case with frame kind 8, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields. PADDED DATA
failures use source-visible
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic(...))` payloads, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields, including the
structured bounded byte preview. The `http2::diagnostic::protocol_invalid_data_padding(...)`
standard helper returns the same source-visible payload directly. A SETTINGS ACK
received while no local SETTINGS batch is
outstanding uses id `http2.protocol.unexpected_settings_ack` and records
`byte_offset.value`, `frame_kind`, `stream_id`, `stream_ref`, `active_state`,
and `rule_provenance`, plus a structured bounded `byte_preview` for the
inspected frame header bytes. The preview uses the same object shape as other
protocol-owned byte previews while SETTINGS ACK state facts stay in their own
fields. The checked human and JSON examples return a source-visible
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic(...))` payload, so
`details.value` keeps the rendered `RuntimeDiagnostic(...)` value while
`details.protocol_diagnostic` keeps the same public fields. The
`http2::diagnostic::protocol_unexpected_settings_ack(...)` standard helper returns the same
source-visible payload directly. A peer-sent `SETTINGS_ENABLE_PUSH` item on a
client receive path uses id
`http2.protocol.settings_not_allowed_for_endpoint` and records
`byte_offset.value`, `setting_identifier`, `setting_name`, `endpoint_role`,
`frame_kind`, `stream_id`, `stream_ref`, `active_state`, and
`rule_provenance`, plus a structured bounded `byte_preview` of the inspected
six-byte SETTINGS item. The checked human and JSON examples return a
source-visible
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolSettingsNotAllowedForEndpointDiagnostic(...))` payload,
and the `http2::diagnostic::protocol_settings_not_allowed_for_endpoint(...)` helper returns
the same value directly. A PRIORITY frame
whose dependency stream id is its own
stream id uses id `http2.protocol.invalid_priority_dependency` and records
`byte_offset.value`, `frame_kind`, `stream_id`, `stream_ref`,
`dependency_stream_id`, `active_state`, and `rule_provenance`, plus a
structured bounded `byte_preview` for the inspected PRIORITY payload bytes.
The checked human and JSON examples return a source-visible
`RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolPriorityDependencyDiagnostic(...))` payload, so
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
Source-visible HPACK static Huffman failures projected from the static
boundary keep the same fields and use `codec_module: "hpack_static"`.
The source-visible HPACK static decoder uses
`hpack.static.unsupported_index` for static-only header blocks that name an
unsupported static-table index; it reuses the same public fields with
`codec_module: "hpack_static"`. Focused checked examples cover both a direct
source-visible `RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDiagnostic(...))` value and projection from
the HTTP/2 protocol-core HPACK failure path. It accepts bounded static-name
literal-without-indexing, literal-with-indexing, and literal-never-indexed
source-visible HPACK inputs for names resolved through the HPACK static table
metadata when their values are raw single-byte-length visible ASCII or a
bounded Huffman-marked literal value decoded through the HPACK static
Huffman table. Focused receive-dispatch and HPACK cases route the checked
Huffman-marked `:scheme: https` request block through completed HEADERS and
final CONTINUATION paths, and route the checked Huffman-marked `:path: test`
static-name literal through the same completed HEADERS and final CONTINUATION
paths. Malformed Huffman padding,
EOS-as-symbol, and non-visible decoded outputs on the promoted static
boundary keep their focused HPACK fixture ids while projecting
`codec_module: "hpack_static"`. Malformed raw lengths and out-of-scope
header blocks stay on the unsupported static header-block fallback path. It
also accepts the
static-table
`content-length` name in literal-without-indexing, literal-with-indexing, and
literal-never-indexed request and response blocks when the raw value is an
accepted visible ASCII decimal string and the block does not require later
fixture dynamic-table reuse; that value feeds the existing content-length
body-accounting state. Non-decimal visible values on the same decoded request
or response path still use the existing header-list rules. The protocol-core
case also checks source-visible HPACK Huffman payload encoding directly for a
supported string, bounded raw bytes, and an unsupported string that returns a
fixture failure. The focused HPACK fixture boundary case checks the same
payload-only boundary directly before the outbound header-list fixture
encoder cases. Stateful HTTP/2
header-block decoding routes supported static-name literal-with-indexing
blocks through the source-visible static decoder and updates carried HPACK
dynamic state for later dynamic-indexed reuse. Unsupported
literal-with-indexing forms still fall back to the HPACK fixture decoder when
the checked source-visible decoders do not own the form.
These diagnostics record
`byte_offset.value`, `observed_header_block_size`,
`observed_first_byte`, `expected_fixture`, and `codec_module`, plus a
structured bounded `byte_preview` for the inspected header-block bytes.
Dynamic indexed lookup failures use id
`hpack.fixture.dynamic_index_out_of_range` and also record
`requested_dynamic_index` and `dynamic_table_entry_count` before the same
expected fixture, codec module, and bounded byte-preview fields. Source-visible
payloads for this id carry those fields in
`RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDynamicIndexDiagnostic(...))`.
The focused HPACK fixture dynamic-index JSON case first accepts a
literal-with-indexing `:path: /target`, then accepts `0xbe` against the
returned fixture state, and finally checks that the same indexed byte reports
the focused out-of-range payload after a bounded-table eviction removes the
entry.
The standalone `hpack_dynamic_core` boundary checks accepted dynamic indexed
paths for multiple carried bounded-table entries, dynamic-core decode-count
advancement after each accepted decode, saturated seven-bit indexed
representations `0xff 0x00` and `0xff 0x80 0x00` resolving HPACK index
`127` to dynamic table index `65`, and the same focused dynamic-index failure
facts without state advancement, including out-of-range
`0xff 0x80 0x01`, in
`examples/specification/run/hpack-fixture-codec-boundary/`. The same boundary
case checks source-visible dynamic-table accounting stdout for the HPACK entry
size formula, newest-first insertion, retained older entries, table-size
reduction eviction including a zero-size table, insertion-caused eviction, and
over-limit insertion. It also checks source-visible static-name
literal-with-indexing `content-type: text`, immutable dynamic-core insertion,
and dynamic-indexed reuse through `0xbe`. It checks accepted raw visible-ASCII
literal-name fields for literal-without-indexing, literal-with-indexing, and
literal-never-indexed, including bounded Huffman-marked values accepted by the
checked HPACK Huffman boundary, dynamic-table mutation only for
literal-with-indexing, dynamic-indexed reuse of the inserted Huffman-valued raw
literal, and focused malformed-Huffman fallback projection.
It also checks source-visible dynamic-name literal receive forms for names
resolved from the carried bounded dynamic table: literal-without-indexing and
literal-never-indexed decode fresh values while retaining the existing entry
for later `0xbe` reuse, and literal-with-indexing inserts `:path: /again`,
reuses it through `0xbe`, and keeps the older `:path: /target` entry
available through `0xbf` when the table has room. The same standalone
boundary checks dynamic-name Huffman-marked values for all three forms:
literal-without-indexing and literal-never-indexed retain the carried dynamic
name entry, literal-with-indexing inserts the decoded Huffman value for later
`0xbe` reuse, and malformed Huffman padding remains on the focused fixture
fallback path.
The fixture keeps prefixed-integer decoding private to its header codec and
does not print a standalone integer transcript. The public finite codec is
specified in `http2.md` and its checked JSON command envelope and stdout are
owned by
`examples/specification/run/hpack-prefixed-integer-codec/`.
The HTTP/2 aggregate case checks completed HEADERS and final CONTINUATION
routing through that same source-visible raw literal-name boundary before
fixture fallback, and completed HEADERS routing through the same
source-visible dynamic indexed boundary before fixture fallback for accepted
multi-continuation and out-of-range dynamic indexed fields. It also checks
completed HEADERS and final CONTINUATION routing for dynamic-name
Huffman-marked values before fixture fallback, including dynamic indexed reuse
of the inserted `:path: test` entry after both routes. Those boundary
checks are ordinary program stdout, not
`details.protocol_diagnostic`, because they do not return a
`RuntimeDiagnostic(...)` payload.
The focused outbound fixture coverage also checks new Huffman literal names
for literal-without-indexing, literal-with-indexing, and
literal-never-indexed with both raw and Huffman values. Its stdout records
exact bytes, empty dynamic tables for the two non-inserting forms, insertion
and `0xbe` reuse for literal-with-indexing, and retained carried state after
unsupported Huffman-name input fails without output. The HTTP/2 aggregate
case routes the indexed `test: ok` block and its later dynamic-index reuse
through outbound HEADERS and reports the returned fixture state.
Missing, malformed, and out-of-range dynamic-name continuations use ids
`hpack.fixture.dynamic_name_continuation_missing`,
`hpack.fixture.dynamic_name_continuation_malformed`, and
`hpack.fixture.dynamic_name_continuation_out_of_range`; their source-visible
payloads carry the same requested dynamic index, dynamic table entry count,
expected fixture, codec module, and bounded byte-preview fields in
`RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDynamicNameDiagnostic(...))`.
When a dynamic table-size update appears after a decoded header field in the
same completed header block, the HPACK fixture boundary uses id
`hpack.fixture.table_size_update_not_at_start` and also records
`observed_header_table_size`, `frame_kind`, `stream_id`, `stream_ref`, and
`active_state` before the same expected fixture, codec module, and byte
preview fields. Source-visible payloads for this id carry those fields in
`RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureTableSizeUpdateDiagnostic(...))`.
When a table-size update integer successfully decodes at the start of a
header block but leaves trailing header-block bytes, the HPACK fixture
boundary uses id `hpack.fixture.table_size_update_trailing_bytes` on the
standalone HPACK fixture boundary and through completed HEADERS and final
CONTINUATION paths. It carries the same table-size update payload fields and
records the decoded table size before the expected fixture, codec module, and
bounded preview.
When a malformed non-terminating table-size update integer is decoded at the
start of a completed header block, the HPACK fixture boundary uses id
`hpack.fixture.table_size_update_malformed`; source-visible payloads for this
id use the common `RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDiagnostic(...))` fields with the
malformed bytes in the bounded preview.
Outbound header-list encode failures in the aggregate HTTP/2 run case stay as
typed HPACK fixture results in program stdout; they are not converted into
`details.protocol_diagnostic`.

Other non-zero Java process exits use `error.kind: "runtime"` with
`details.phase: "runtime"`. JDK setup failures use `error.kind: "runner"` with
`details.phase: "tool"`.
