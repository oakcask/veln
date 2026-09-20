---
role: specification
authority: normative
update-when: The `veln run --json` envelope, runtime failure projection, output capture, or checked run JSON contract changes.
specification-coverage: usage=#usage; behavior=#returned-result-values; limits=#output-and-limits
---

# Run JSON

`veln run --json` selects and executes one entry. It emits one JSON document
unless analysis fails before the run report can be constructed, in which case it
emits the shared diagnostic envelope.

## Usage

Run a project entry with `veln run --json ENTRY [PATH ...] [-- ARG ...]` (or the
command's normal entry-selection options) and parse the single JSON document
from stdout. `ENTRY` is required; `PATH` supplies optional source inputs and
arguments after `--` are passed to the entry.
For example, a successful result begins:

```json
{"schema_version":"veln-run-json/v0","command":"run","status":"passed","exit_code":0,"stdout":"","stderr":"","error":null}
```

The example shows the required envelope fields; output strings and error
details depend on the selected entry.

## Envelope

A completed run uses schema version `veln-run-json/v0` and these fields:

| Field | Values and meaning |
| --- | --- |
| `command` | Always `"run"`. |
| `status` | `"passed"`, `"failed"`, or `"error"`. |
| `exit_code` | Captured Java process status, or `1` for a tool error. |
| `stdout` | Captured program standard output. |
| `stderr` | Captured program standard error. |
| `error` | `null` for a passed run; a structured error otherwise. |

A passed run has a null `error`. A non-passed run causes the CLI to exit
unsuccessfully. A normal runtime failure preserves the program output captured
before the failure. A transport failure replaces the captured stderr with its
stable transport message followed by a newline; raw Java stderr is discarded.

A tool failure has `error.kind: "runner"`, `details.phase: "tool"`, empty
`stdout` and `stderr`, and does not claim that the program reached the backend.
Other non-zero Java exits have `error.kind: "runtime"` and
`details.phase: "runtime"`.

## Static diagnostic gate

Parse, source, semantic, lowering, and run-entry effect diagnostics stop the
run before artifact generation or Java launch. The command then emits the
shared diagnostic envelope with `schema_version: 1`, `status: "error"`,
`diagnostics`, and `summary`; this pre-execution envelope has no captured
program `stdout` or `stderr` fields. Any CLI diagnostic rendering on stderr is
separate from the JSON contract.

The gate examines the selected entry closure. It includes reachable
declarations, aliases, type and constructor paths, handler bindings and
clauses, written imports, source-path-derived module identities, and loaded
direct dependencies. An invalid path or declaration outside that closure, an
unused import path, an unselected source path, or an unloaded manifest
dependency does not block an otherwise valid selected entry. Recovery
ambiguity remains a diagnostic error and is not converted into a recovered
name.

## Runtime failures

A contract failure has `error.kind: "contract"` and details:

| Field | Values |
| --- | --- |
| `kind` | `"contract"`. |
| `phase` | `"runtime"`. |
| `clause` | `"require"`, `"ensure"`, or `"invariant"`. |
| `predicate` | Text of the failed clause. |
| `function` | Checked function boundary. |
| `blame` | `"caller"` for `require`, `"implementation"` for `ensure`; entry or return ownership for `invariant`. |
| `node_id` | Contract node identifier. |
| `span` | Source span of the failed clause. |

A host transport failure has `error.kind: "runtime"`,
`details.id: "runtime.transport_failure"`, and these observed facts:

- `operation`, `category`, and `lifecycle_phase`;
- optional `local_endpoint`, `peer_endpoint`, `listener_id`, and
  `stream_id`;
- optional `input_committed`, `output_committed`, and
  `ownership_committed`;
- optional `platform_cause`.

Unknown facts are omitted. The primary error message contains the failed
operation and stable category. An invalid dynamic shift count uses
`details.id: "runtime.invalid_shift_count"` with `operator`,
`actual_count`, `minimum_count`, and `maximum_count`. Other host failures
keep the generic runtime shape.

## Returned result values

An entry returning `Err(value)` has `error.kind: "result"` and details
`kind: "result"`, `phase: "runtime"`, and the rendered returned value in
`value`. The rendered value is retained even when typed details are projected
from it. A plain value without a recognized diagnostic form does not acquire
typed fields.

### Compact hexadecimal details

A compact hexadecimal input failure may add `fixture_hex`:

| Field | Meaning |
| --- | --- |
| `kind` | `"fixture_hex"`. |
| `id` | `"fixture.hex.invalid_character"` or `"fixture.hex.odd_length"`. |
| `fixture_text_span` | Start and end offsets in the input text. |
| `byte_offset` | Decoded `ByteOffset` kind and value. |
| `nibble_position` | `"high"` or `"low"`. |
| `nearby_context` | Bounded text surrounding the failure. |

Valid bytes that are too short for a closed-input read remain ordinary
truncation and do not receive `fixture_hex`.

### Byte diagnostic projection

A source-visible `RuntimeDiagnostic(..., RuntimeByteDiagnostic(...))`
projects `details.byte_diagnostic` while preserving `details.value`. Its
common fields are:

- `kind: "byte_diagnostic"`, `id`, and `message`;
- `byte_offset`, `field_path`, and optional `field_path_display`; field-path
  segments carry `kind` and `name`, with `index` segments for repeated fields;
- one typed fact: count/readiness, range, fixed value, or `reason`;
- optional bounded `byte_preview`.

Count facts use `expected_count`, `available_count`, and `readiness`.
Range facts use `requested_count` and `available_count`. Fixed-value facts
use `expected_value` and `actual_value`. Helper context may add
`local_byte_offset`, `expected_count`, `available_count`, and
`byte_preview`. The preview has `encoding: "hex"`, lowercase `data`,
`preview_byte_count`, `total_byte_count`, and `truncated`.

A closed binary dispatch failure uses `id: "schema.dispatch_unknown_tag"`,
the dispatch `tag_field`, `decoded_tag_value`, `expected_tags`, the
decoded byte offset and field path, and the same preview object.

Generated binary schema byte diagnostics use these additional ids:

| ID | Additional fields |
| --- | --- |
| `schema.fixed_field_mismatch` | `expected_value`, `actual_value`. |
| `schema.truncated_field` | `expected_count`, `available_count`, `readiness: "need_bytes"`; repeated fields add an index segment to `field_path`. |
| `schema.length_out_of_bounds` | `expected_count`, `available_count`, `readiness: "need_bytes"`. |
| `schema.length_multiple_mismatch` | `observed_count`, `required_multiple`, `multiple_operand`. |
| `schema.length_division_by_zero` | `length_expression`, `operator`, `divisor_operand`. |
| `schema.integer_out_of_range` | `byte_width`, `min_value`, `max_value`, and `actual_value` or `actual_value_text`. |
| `schema.reserved_bits_mismatch` | `bit_width`, `expected_value`, `actual_value`. |
| `schema.validation_failed` | Predicate owner offset, `predicate`, `decoded_values`; length diagnostics may also expose `length`, `padding_length`. |

These ids use schema-local field-path segments and the common bounded preview
where bytes were inspected. A field-local validation offset identifies the
owning field; a schema-level validation offset follows the decoded body.

### Value diagnostic projection

A source-visible `RuntimeValueDiagnostic` projects `details.value_diagnostic`.
Generated encode failures retain the rendered value and expose schema-local
`field_path`, `field_path_display`, and `reason`. Schema validation and
checked `byte_write_*` conversion failures use the same value projection.
Generated value diagnostics use `schema.encode_value_unrepresentable`,
`schema.dispatch_unknown_tag`, `schema.dispatch_length_mismatch`,
`schema.dispatch_mismatch`, or `schema.validation_failed`. Length mismatches
may add `expected_count`, `actual_count`, `length_expression`, `byte_offset`,
and `byte_preview`; validation failures may add `predicate`, `field_value`,
and `supplied_values`. Hand-written codec values may use the corresponding
`codec.encode_value_unrepresentable`, `codec.dispatch_unknown_tag`,
`codec.dispatch_length_mismatch`, and `codec.dispatch_mismatch`. A byte
write value diagnostic uses `helper_name`, `supplied_value`, `min_value`,
`max_value`, `width`, and `byte_order` (`"big_endian"` or `"little_endian"`).
Encode step values that represent successful or partial encoding do not produce
an error projection.

## Codec diagnostics

Codec diagnostics use `details.byte_diagnostic` and preserve the common byte
fields above. The reason parser adds fields only when the source-visible reason
has the matching valid form; plain or malformed reason text remains only in
`reason`.

The supported reason forms are exact key/value text with semicolon separators:

| ID | Parsed reason keys |
| --- | --- |
| `codec.checksum_mismatch` | `expected_checksum=<value>; actual_checksum=<value>; reason=<text>` |
| `codec.length_mismatch` | `expected_length=<n>; actual_length=<n>; reason=<text>` |
| `codec.payload_length_mismatch` | `expected_payload_length=<n>; actual_payload_length=<n>; reason=<text>` |
| `codec.padding_mismatch` | `expected_padding_length=<n>; actual_padding_length=<n>; reason=<text>` |
| `codec.integer_out_of_range` | `byte_width=<n>; min_value=<n>; max_value=<n>; actual_value=<n>; reason=<text>` |
| `codec.sequence_mismatch` | `expected_sequence=<value>; actual_sequence=<value>; reason=<text>` |
| `codec.version_mismatch` | `expected_version=<value>; actual_version=<value>; reason=<text>` |
| `codec.tag_mismatch` | `expected_tag=<value>; actual_tag=<value>; reason=<text>` |
| `codec.magic_mismatch` | `expected_magic=<value>; actual_magic=<value>; reason=<text>` |
| `codec.unsupported_feature` | `feature=<value>; reason=<text>` |
| `codec.trailing_input` | `consumed_count=<n>; available_count=<n>; remaining_count=<n>; reason=<text>` |
| `codec.consumed_count_invalid` | `available_count=<n>; actual_consumed_count=<n>; reason=<text>` |

Numeric forms are projected only when their values are valid and consistent;
otherwise the original `reason` is retained without inferred fields.

| ID | Conditional fields |
| --- | --- |
| `codec.packet_kind_invalid` | Registered helper context only. |
| `codec.checksum_mismatch` | `expected_checksum`, `actual_checksum`. |
| `codec.length_mismatch` | `expected_length`, `actual_length`. |
| `codec.payload_length_mismatch` | `expected_payload_length`, `actual_payload_length`. |
| `codec.padding_mismatch` | `expected_padding_length`, `actual_padding_length`. |
| `codec.integer_out_of_range` | `byte_width`, `min_value`, `max_value`, `actual_value`. |
| `codec.sequence_mismatch` | `expected_sequence`, `actual_sequence`. |
| `codec.version_mismatch` | `expected_version`, `actual_version`. |
| `codec.tag_mismatch` | `expected_tag`, `actual_tag`. |
| `codec.magic_mismatch` | `expected_magic`, `actual_magic`. |
| `codec.unsupported_feature` | `unsupported_feature`. |
| `codec.trailing_input` | `consumed_count`, `available_count`, `remaining_count` only when nonnegative and consistent. |
| `codec.consumed_count_invalid` | `available_count`, `actual_consumed_count`. |
| `codec.incomplete_input` | `readiness: "need_bytes"`; `needed_count` for `NeedBytes`, omitted for `NeedEnd`. |
| `codec.byte_range_out_of_bounds` | Source range and availability facts. |
| `codec.invalid_input` | Source reason and registered helper context. |

The same projection applies to direct result values and invalid decode-step
values. A decoded value with no error does not populate `error`.

## HTTP/2 and HPACK diagnostics

HTTP/2 protocol errors are ordinary source-level values until an explicit
projection reports them. A projected value retains the rendered
`RuntimeDiagnostic(...)` in `details.value` and exposes the same inner
fields in `details.protocol_diagnostic`. The common fields are wire
`byte_offset.value` when available, frame and stream identity when known,
including `frame_kind`, `stream_id`, and `stream_ref` when supplied,
`active_state`, `rule_provenance`, and a bounded `byte_preview` when bytes
were inspected.

| ID | Variant fields |
| --- | --- |
| `http2.protocol.partial_preface` | `pending_count`, `expected_count`. |
| `http2.protocol.invalid_preface` | `expected_byte`, `actual_byte`, `matched_prefix_count`, `expected_count`. |
| `http2.protocol.closed_with_pending` | `pending_count`, `input_event`, `active_continuation`, `expected_stream_id`, `started_frame_kind`, `started_byte_offset`, `accumulated_header_block_bytes`. |
| `http2.protocol.continuation_expected` | `actual_frame_kind`, `actual_stream_id`, `expected_stream_id`, `started_frame_kind`, `started_byte_offset`, `active_continuation`, `accumulated_header_block_bytes`. |
| `http2.peer_limit.frame_size_exceeded` | `observed_payload_length`, `allowed_max_frame_size`, `receive_limit_provenance`. |
| `http2.peer_limit.settings_value_out_of_range` | `setting_identifier`, `setting_name`, `observed_value`, `accepted_min_value`, `accepted_max_value`, `peer_limit_provenance`. |
| `http2.peer_limit.flow_control_window_exceeded` | `observed_payload_length`, `allowed_window_credit`. |
| `http2.protocol.invalid_window_update_increment` | `observed_window_increment`, `accepted_min_window_increment`, `accepted_max_window_increment`. |
| `http2.protocol.invalid_data_padding` | `pad_length`, `remaining_payload_length`. |
| `http2.protocol.content_length_mismatch` | `expected_content_length`, `observed_body_length`. |
| `http2.peer_limit.concurrent_streams_exceeded` | `current_open_peer_created_stream_count`, `attempted_concurrent_stream_count`, `allowed_concurrent_stream_count`, `endpoint_role`, `receive_limit_provenance`. |
| `http2.peer_limit.header_list_size_exceeded` | `observed_header_list_size`, `allowed_header_list_size`, `receive_limit_provenance`. |
| `http2.peer_limit.header_table_size_exceeded` | `observed_header_table_size`, `allowed_header_table_size`, `receive_limit_provenance`. |
| `http2.protocol.invalid_request_header_list` | `failed_header_fact`, `header_name`, `decoded_header_names`. |
| `http2.protocol.invalid_response_header_list` | `failed_header_fact`, `header_name`, `decoded_header_names`. |
| `http2.protocol.invalid_frame_kind` | `actual_frame_kind`, `expected_frame_kind`. |
| `http2.protocol.invalid_stream_id` | `frame_kind`, `required_stream_id_domain`, `endpoint_role`. |
| `http2.protocol.peer_stream_id_not_increasing` | `previous_peer_stream_id`, `endpoint_role`. |
| `http2.protocol.initial_peer_settings_required` | `actual_frame_kind`, `actual_flags`, `endpoint_role`. |
| `http2.protocol.unexpected_settings_ack` | Settings acknowledgement state. |
| `http2.protocol.invalid_payload_length` | `frame_kind`, `observed_payload_length`, `expected_payload_length`. |
| `http2.protocol.invalid_priority_dependency` | `frame_kind`, `dependency_stream_id`. |
| `http2.protocol.stream_after_goaway` | `last_stream_id`, `shutdown_state`, `endpoint_role`. |
| `http2.protocol.settings_not_allowed_for_endpoint` | `setting_identifier`, `setting_name`, `endpoint_role`, `frame_kind`. |

HPACK diagnostics use these ids and fields in addition to the common protocol
projection. The shared HPACK fields are `byte_offset`,
`observed_header_block_size`, `observed_first_byte`, `expected_fixture`,
`codec_module`, and the bounded `byte_preview`.

| HPACK id | Additional fields |
| --- | --- |
| `hpack.fixture.unsupported_header_block` | Shared HPACK fields. |
| `hpack.fixture.unsupported_static_index` | Shared HPACK fields. |
| `hpack.static.unsupported_index` | Shared HPACK fields. |
| `hpack.fixture.malformed_string_length` | Shared HPACK fields. |
| `hpack.fixture.malformed_raw_string_value` | Shared HPACK fields. |
| `hpack.fixture.malformed_huffman_padding` | Shared HPACK fields. |
| `hpack.fixture.huffman_eos_symbol` | Shared HPACK fields. |
| `hpack.fixture.huffman_non_visible_value` | Shared HPACK fields. |
| `hpack.fixture.table_size_update_malformed` | Shared HPACK fields. |
| `hpack.fixture.dynamic_index_out_of_range` | `requested_dynamic_index`, `dynamic_table_entry_count`, plus shared fields. |
| `hpack.fixture.dynamic_name_continuation_missing` | `requested_dynamic_index`, `dynamic_table_entry_count`, plus shared fields. |
| `hpack.fixture.dynamic_name_continuation_malformed` | `requested_dynamic_index`, `dynamic_table_entry_count`, plus shared fields. |
| `hpack.fixture.dynamic_name_continuation_out_of_range` | `requested_dynamic_index`, `dynamic_table_entry_count`, plus shared fields. |
| `hpack.fixture.table_size_update_not_at_start` | `observed_header_table_size`, `frame_kind`, `stream_id`, `stream_ref`, `active_state`, plus shared fields. |
| `hpack.fixture.table_size_update_trailing_bytes` | `observed_header_table_size`, `frame_kind`, `stream_id`, `stream_ref`, `active_state`, plus shared fields. |

For header-list diagnostics, `decoded_header_names` is the decoded-name list
and `byte_preview` covers the inspected header block. Receive-limit
provenance refers to the active local receive policy. Peer-advertised SETTINGS
values remain outbound state and are not substituted for local receive limits.
Unknown SETTINGS identifiers do not update peer state. Known duplicate
SETTINGS values are processed in wire order; invalid later updates leave the
previous peer state unchanged.

Accepted outbound send intents and adapter summaries remain ordinary program
output. They do not populate `error` or `details.protocol_diagnostic`.
A protocol value returned without an explicit projection remains an ordinary
result failure.

## Output and limits

The report captures user-program output as strings available at the time of
failure. Transport failures use the replacement stderr rule above. It does not
retry failed host operations, infer unknown ownership or commit facts, or add
typed fields to unrecognized returned values.

The report implementation is `crates/veln-cli/src/commands/run_report.rs`;
entry selection and execution are implemented in
`crates/veln-cli/src/commands/run.rs`. The CLI JSON assertions under
`crates/veln-cli/tests/check_json/run.rs` are the executable verification
route for the envelope and projections.
