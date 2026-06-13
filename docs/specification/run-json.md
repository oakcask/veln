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

When the result value is a schema fixed-field mismatch,
`details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "schema.fixed_field_mismatch"`
- `byte_offset`: the decoded-stream `ByteOffset` of the mismatched field
- `field_path`: schema-local path segment objects with `kind` and `name`
- `expected_value`: the fixed byte value required by the schema field
- `actual_value`: the decoded byte value that was present
- `byte_preview`: a structured bounded byte preview object

When the result value is a binary schema frame-header truncation,
`details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "schema.truncated_field"`
- `byte_offset`: the first missing decoded-stream `ByteOffset`
- `field_path`: schema-local path segment objects with `kind` and `name`
- `expected_count`: the required field byte count
- `available_count`: the byte count available for that field
- `readiness: "need_bytes"`
- `byte_preview`: a structured bounded byte preview object

When the result value is a binary schema payload length boundary failure,
`details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "schema.length_out_of_bounds"`
- `byte_offset`: the first missing decoded-stream `ByteOffset`
- `field_path`: schema-local path segment objects with `kind` and `name`
- `expected_count`: the decoded payload length
- `available_count`: the byte count available after the frame header
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

When the result value is a binary schema field-local validation failure,
`details.byte_diagnostic` includes:

- `kind: "byte_diagnostic"`
- `id: "schema.validation_failed"`
- `byte_offset`: the decoded-stream `ByteOffset` of the field that owns the
  failed predicate
- `field_path`: schema-local path segment objects with `kind` and `name`
- `predicate`: the failed field-local `where` predicate text
- `field_value`: the decoded value of the owning field
- `decoded_values`: display text for decoded schema fields available to the
  predicate
- decoded field values available to the predicate, keyed by schema field name,
  such as `length` and `padding_length`
- `byte_preview`: a structured bounded byte preview object

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
- `data`: lowercase hex byte pairs for the previewed bytes
- `preview_byte_count`: the number of bytes present in `data`
- `total_byte_count`: the total byte count represented by the preview source
- `truncated`: whether `data` is a shortened prefix of the preview source

Named binary fixture cases can assert the same byte-stream facts after a
fixture record decodes successfully. Invalid compact hex remains a
`details.fixture_hex` failure. Valid fixture bytes that are too short for a
closed-input read remain ordinary codec truncation without fixture hex details.
Valid fixture bytes that fail a test-owned codec or protocol field check use
fixture metadata for the diagnostic id, byte offset, structured field path,
and consumed count where applicable.

HTTP/2 protocol-core failures that originate from a source-visible projection
helper attach `details.protocol_diagnostic`. End-of-stream with a partial
client connection preface uses id `http2.protocol.partial_preface` and records
`byte_offset.value`, `pending_count`, `expected_count`, `active_state`, and
`rule_provenance`. A mismatched client connection preface byte uses id
`http2.protocol.invalid_preface` and records `byte_offset.value`,
`expected_byte`, `actual_byte`, `matched_prefix_count`, `expected_count`,
`active_state`, and `rule_provenance`. The frame-size peer-limit slice uses id
`http2.peer_limit.frame_size_exceeded` and records
`byte_offset.value`, `observed_payload_length`, `allowed_max_frame_size`,
`frame_kind`, `stream_id`, `stream_ref`, and `receive_limit_provenance`. The
provenance names the active receive-limit entry used for the failed inbound
frame-size check, such as protocol default, local configuration, or local
SETTINGS. Peer-received `SETTINGS_MAX_FRAME_SIZE` and
`SETTINGS_INITIAL_WINDOW_SIZE` values belong to peer-advertised state for
outbound decisions and are not reported as the receive-limit provenance for
later inbound frame-size or DATA receive-window failures. Received
DATA frames that exceed available inbound receive-window credit, and
`WINDOW_UPDATE` increments that would exceed available inbound receive-window
growth, use id
`http2.peer_limit.flow_control_window_exceeded` and record
`byte_offset.value`, `observed_payload_length`, `allowed_window_credit`,
`frame_kind`, `stream_id`, `stream_ref`, `active_state`, and
`rule_provenance`; the checked HTTP/2 examples cover both stream-window and
connection-window receive credit failures. The ordinary protocol-core example
also covers zero `WINDOW_UPDATE` increments as peer-limit failures. Received
SETTINGS range failures use id
`http2.peer_limit.settings_value_out_of_range` and record
`byte_offset.value`, `setting_identifier`, `setting_name`, `observed_value`,
`accepted_min_value`, `accepted_max_value`, and `peer_limit_provenance`. The
invalid frame-kind state slice uses id `http2.protocol.invalid_frame_kind` and
records `byte_offset.value`, `actual_frame_kind`, `stream_id`, `stream_ref`,
`expected_frame_kind`, `active_state`, and `rule_provenance`; the checked
HTTP/2 examples cover both connection-control and idle-stream state failures.
Wrong-length protocol payloads use id
`http2.protocol.invalid_payload_length` and record `byte_offset.value`,
`frame_kind`, `stream_id`, `stream_ref`, `observed_payload_length`,
`expected_payload_length`, `active_state`, and `rule_provenance`; the checked
HTTP/2 examples cover the PING fixed-length failure, GOAWAY fixed-prefix
length failure, and `WINDOW_UPDATE` fixed-length failure.

Other non-zero Java process exits use `error.kind: "runtime"` with
`details.phase: "runtime"`. JDK setup failures use `error.kind: "runner"` with
`details.phase: "tool"`.
