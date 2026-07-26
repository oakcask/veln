# HTTP/2 Standard Library Completion and Fixture Retirement

Status: proposed

This proposal defines the finite work required to retire the monolithic
`../../examples/specification/run/http2-protocol-core/` case. Current
implemented behavior belongs under `../specification/`; the implemented public
module boundary is summarized in `../specification/http2.md`.

## Problem

HTTP/2 is now an opt-in standard-library feature. Public frame, diagnostic,
HPACK, and stream-domain entry points exist under `std::http2`, and ordinary
projects load only the imported standard-module dependency closure.

Most protocol behavior and its strongest executable evidence still live in one
specification case instead of the standard package. That case currently owns:

- connection, stream-lifecycle, SETTINGS, flow-control, header-validation, and
  graceful-shutdown transitions;
- the production HPACK codec, dynamic-table state, header-field model, and
  fixture-specific compatibility paths;
- 65 `require_*` helper definitions used by 717 call sites;
- one complete stdout value and 315 output-chunk-list assertions; and
- local modules whose responsibilities overlap the new standard modules.

The fixture takes materially longer to analyze and execute than focused cases.
Deleting it now would remove coverage. Keeping it indefinitely would leave the
standard API incomplete and continue to make unrelated workspace verification
pay for a monolithic integration case.

## Target Boundary

The completed design has one implementation owner for each reusable behavior:

- `std::http2::frame` owns frame schema, decode, and encode behavior.
- `std::http2::diagnostic` owns protocol and peer-limit diagnostic conversion.
- `std::http2::hpack` owns the public HPACK codec and immutable state API.
- `std::http2::hpack::diagnostic` owns HPACK diagnostic conversion.
- `std::http2::core` owns sans-I/O connection and stream transitions.
- adjacent standard-library `*_test.veln` files own pure unit coverage.
- focused cases under `../../examples/specification/` own observable CLI,
  human-diagnostic, JSON, result-value, and output-chunk coverage.

The monolithic case is not an implementation module and is not a permanent
test utility. It may be deleted only after every behavior and observable
assertion has a classified replacement.

## Finite Remaining Scope

### HPACK Library Completion

Promote reusable behavior from `hpack_fixture.veln`, `hpack_static.veln`, and
`hpack_dynamic_core.veln` into responsibility-named private modules behind the
`std::http2::hpack` facade:

- integration of the implemented immutable dynamic table with production
  header representation encode transitions;
- all representation encoding;
- ordered-list encoding with octet-preserving values; and
- immutable encode transitions that expose no partial output or next state
  after failure.

Fixture display labels, canned header lists, stdout formatting, and expected
value construction do not belong in the standard package. Public HPACK names
must not retain the `hpack_fixture_` prefix.

The stateless HPACK Huffman codec is complete under `std::http2::hpack`.
Its public facade accepts arbitrary octets, preserves non-visible decoded
values, applies EOS-prefix padding, and rejects EOS, invalid padding, and
truncated or invalid codes without exposing partial output. The former public
fixture byte-label facade and its private adapter intrinsics are removed.
Indexed, literal, table-size-update, and complete ordered header-block decoding
are implemented behind the public facade. Literal decoding covers incremental
indexing, without indexing, and never indexed; direct and indexed names; raw
and Huffman strings; and immutable success and failure transitions. Complete
block decoding composes those codecs in wire order, permits only bounded
leading size updates, and exposes no partial list or next table after failure.
Production encoding is also complete behind the public facade. Indexed and all
three literal forms support direct, static, and dynamic names, raw and Huffman
strings, exact value octets, and immutable table transitions. Ordered-list
encoding applies a deterministic exact-entry, name-index, and shortest-string
policy; later fields see successful insertions from earlier fields. Validation,
index lookup, integer or string encoding, table transition, and active-capacity
selection failures are focused typed results with no partial bytes or next
state.

Residual `hpack.fixture.unsupported_header_block` references are confined to
the legacy fixture diagnostic boundary, compatibility projections, fixture
decode fallbacks, and fixture-owned canned encoders in the monolithic
protocol-core case. They are not production HPACK encode fallbacks. Their
removal belongs to the remaining sans-I/O core and evidence migration below,
so the fixture and this proposal stay in place.

### Sans-I/O Core Completion

Promote the pure protocol state and transitions from `main.veln` and
`stream_domain.veln` into responsibility-named private modules behind
`std::http2::core`:

- connection preface and the initial peer SETTINGS gate;
- frame-kind, payload-length, and stream-id-domain validation;
- client, server, connection, promised, and peer-created stream domains;
- stream lifecycle, priority, reset, reservation, and monotonic peer stream
  admission;
- local and peer SETTINGS state, acknowledgement tracking, and value ranges;
- connection and per-stream receive and send flow-control windows;
- request, response, trailer, informational, no-content, CONNECT, and extended
  CONNECT header validation;
- content-length accounting;
- inbound HEADERS, CONTINUATION, DATA, PRIORITY, RST_STREAM, PUSH_PROMISE,
  SETTINGS, PING, WINDOW_UPDATE, and GOAWAY transitions;
- outbound DATA, HEADERS, PUSH_PROMISE, SETTINGS, PING, WINDOW_UPDATE,
  RST_STREAM, PRIORITY, and GOAWAY transitions; and
- graceful-shutdown admission and drain behavior.

Every transition is sans-I/O: it consumes explicit input and immutable state
and returns an action, next state, or typed failure. A rejected transition
preserves input state, HPACK state, flow-control credit, pending continuation,
and output bytes. Diagnostic precedence remains the same as the current
executable cases.

### Evidence Migration

Maintain a checked migration matrix while implementing this proposal. The
matrix has one row for every `require_*` call site, complete-stdout assertion,
and output-chunk-list assertion in the monolithic case. Each row records:

- the behavior or observable value it protects;
- its destination standard test or focused specification case;
- whether it covers success, failure, or failure-state preservation; and
- whether it protects a diagnostic id, human rendering, structured JSON,
  raw result value, CLI parsing, or emitted bytes.

Rows may be consolidated only when one parameterized or recursive test proves
the same invariant. Consolidation must name the shared invariant; it must not
discard a distinct endpoint role, starting state, transition, diagnostic
precedence rule, or output projection.

The following checked migration matrix records completed slices alongside the
remaining fixture-owned integration evidence. Each row identifies the owning
adjacent standard-library test or focused specification case while retaining
fixture projections that still protect distinct integration or observable
output.

| Source helper or assertion family | Migrated invariant | Destination | Coverage |
| --- | --- | --- | --- |
| `preface-valid`, `preface-partial`, `preface-closed`, and `preface-mismatch` assertions | the 24-octet client connection preface accepts complete, byte-by-byte, and unevenly chunked input; retains matched prefixes; preserves trailing input; reports exact first, middle, and final mismatches or partial closure; and leaves immutable input state unchanged without failure output | `connection_preface_accepts_complete_input_and_preserves_trailing_bytes`, `connection_preface_accepts_byte_by_byte_input`, `connection_preface_accepts_uneven_chunks_and_retains_matched_prefix`, `connection_preface_reports_first_middle_and_final_mismatches`, `connection_preface_close_reports_partial_input`, `connection_preface_failures_preserve_immutable_input_state`, `http2-protocol-core-preface-invalid-human`, `http2-protocol-core-preface-invalid-json`, `http2-protocol-core-preface-partial-human`, `http2-protocol-core-preface-partial-json` | pure success, failure, diagnostic-input, input-state, and failure-output preservation moved to `core_test.veln`; focused cases own human and JSON projection through `http2::diagnostic`; the monolithic assertions remain for initial-SETTINGS integration and complete stdout |
| initial peer SETTINGS acceptance, rejection, role, failure-context, state-preservation, waiting, frame-size, stream-id, payload, and value assertions | the first complete peer frame is accepted for server and client roles only when it is non-ACK SETTINGS; rejection retains the stable failure id and exact frame and connection context without a next state or input mutation | `initial_peer_settings_gate_accepts_server_and_client_roles`, `initial_peer_settings_gate_accepts_without_changing_input_state_or_preview`, `initial_peer_settings_gate_preserves_role_in_successful_next_state`, `initial_peer_settings_gate_rejects_non_settings_and_ack_frames`, `initial_peer_settings_gate_failure_preserves_exact_context`, `initial_peer_settings_gate_failure_preserves_input_state_and_preview`, `http2-initial-peer-settings-gate-human`, `http2-initial-peer-settings-gate-json` | pure role, success, rejection, diagnostic-input, input-state, preview, and failure-output preservation moved to `core_test.veln`; focused cases obtain the public typed failure and own human and JSON projection; the monolithic case calls the public gate while retaining incomplete-input, frame-size, stream-id, payload, SETTINGS-value, state, and complete-stdout integration |
| `stream_domain.veln` stream-id and stream-reference assertions | client and server stream-id domains accept their lower bounds, retain validated values, reject zero and one above the HTTP/2 upper bound for each endpoint role, reject wrong endpoint parity, and project connection and real stream references through the public facade | `stream_domains_are_role_aware`, `stream_domains_project_connection_and_real_references`, `stream_domains_accept_lower_bounds_and_reject_invalid_boundaries`, retained `http2-protocol-core` stream-id diagnostics, complete stdout, and transition ordering | pure role-specific domain and projection assertions moved to `core_test.veln`; the duplicate fixture module is removed; the monolithic case now uses `std::http2::core` directly and retains observable stream-id diagnostic and integration coverage |
| `continuation-more`, `continuation-extra`, `continuation-done`, `continuation`, `continuation-stream`, `unknown-continuation`, `continuation-closed`, and pure PUSH_PROMISE continuation assertions | immutable pending state preserves initiating metadata and octets; HEADERS and PUSH_PROMISE fragments complete only in wire order; idle closure succeeds; wrong-kind, wrong-stream, and closed-input failures preserve diagnostic input, expose no next state or completed block, and preserve the input | `pending_header_block_completes_immediate_headers`, `pending_header_block_completes_multi_frame_headers_in_wire_order`, `pending_header_block_completes_multi_frame_push_promise`, `pending_header_block_preserves_non_final_accumulation_and_metadata`, `pending_header_block_rejects_wrong_kind_and_stream_without_next_output`, `pending_header_block_rejects_closed_input_without_changing_state`, `pending_header_block_accepts_closed_input_while_idle`, `pending_header_block_failures_preserve_diagnostic_input`, `http2-protocol-core-continuation-human`, `http2-protocol-core-continuation-json`, `http2-protocol-core-continuation-stream-json`, `http2-protocol-core-continuation-closed-human`, `http2-protocol-core-continuation-closed-json` | pure success, failure, diagnostic-input, input-state, and failure-output preservation moved to `core_test.veln`; focused cases own human and JSON projection; the monolithic assertions remain only for decoded-frame, HPACK, stream-lifecycle, complete stdout, and output-chunk integration |
| PING, GOAWAY, WINDOW_UPDATE, SETTINGS, RST_STREAM, PRIORITY, HEADERS, PUSH_PROMISE, DATA, CONTINUATION, and unknown-kind payload-length assertions | the complete bounded frame-shape matrix validates fixed, minimum, SETTINGS-item, and flag-selected prefix boundaries; failures retain exact coordinates, frame data, caller-supplied protocol context, rule provenance, and preview without a partial result or input mutation | `frame_payload_length_validates_fixed_and_minimum_boundaries`, `frame_payload_length_validates_settings_ack_and_item_boundaries`, `frame_payload_length_validates_headers_prefix_flag_matrix`, `frame_payload_length_validates_push_promise_prefix_flag_matrix`, `frame_payload_length_leaves_unconstrained_kinds_unrestricted`, `frame_payload_length_failure_preserves_exact_preview_without_output`; `http2-protocol-core-ping-length-human`, `http2-protocol-core-ping-length-json`, `http2-protocol-core-goaway-length-human`, `http2-protocol-core-goaway-length-json`, `http2-protocol-core-window-update-length-human`, `http2-protocol-core-window-update-length-json`, `http2-protocol-core-settings-ack-length-human`, `http2-protocol-core-settings-ack-length-json`, `http2-protocol-core-settings-item-length-human`, `http2-protocol-core-settings-item-length-json`, `http2-protocol-core-rst-stream-length-human`, `http2-protocol-core-rst-stream-length-json`, `http2-protocol-core-priority-length-human`, `http2-protocol-core-priority-length-json`, `http2-protocol-core-headers-prefix-length-human`, `http2-protocol-core-headers-prefix-length-json`, `http2-protocol-core-push-promise-prefix-length-human`, and `http2-protocol-core-push-promise-prefix-length-json` | pure success, boundary failure, diagnostic-input, input-state, and failure-output preservation moved to `core_test.veln`; each fixed, minimum, SETTINGS-item, HEADERS-prefix, and PUSH_PROMISE-prefix failure has focused human and JSON projection through the public failure, including its stored preview; monolithic padding, header decode, stream-id, frame-size, lifecycle, precedence, and complete-stdout assertions remain as integration evidence |
| local PING request, received non-ACK PING, received PING ACK, PING payload-length rejection, and `http2-outbound-ping`, `http2-outbound-ping-short`, `http2-outbound-ping-long`, `http2-ping-ack`, and `http2-ping-ack-received` output-chunk assertions | the public core facade owns exact outbound request bytes, seven- and nine-octet rejection through the shared payload-length failure, empty failure output, immutable failure input, exact ACK bytes with unchanged payload, and explicit no-response behavior for received ACKs | `ping_request_accepts_exact_payload_bytes`, `ping_request_rejects_seven_and_nine_octets_without_output`, `ping_request_failure_preserves_input_and_failure_preview`, `ping_response_ack_preserves_payload_and_emits_exact_bytes`, `ping_response_ack_received_has_no_response`, `http2-core-ping-transitions`, and retained aggregate output-chunk assertions in `http2-protocol-core` | pure request, ACK, no-response, failure typing, failure-output, and immutable-input coverage moved to `core_test.veln`; focused external-package output-chunk and public projection evidence moved to `http2-core-ping-transitions`; monolithic receive ordering, decoded-frame integration, and complete stdout remain as integration evidence |
| local SETTINGS outstanding-batch queue, peer SETTINGS ACK intent, unexpected SETTINGS ACK failure, `http2-settings-ack`, `http2-peer-settings-ack`, and `http2-peer-settings-ack-coalesced` output-chunk assertions | immutable SETTINGS acknowledgement state starts empty, records local batches in FIFO order, rejects unexpected peer ACKs with stable protocol context and no next state, records and coalesces one pending peer ACK intent, returns no-pending without bytes, emits exact ACK bytes, returns an encode-error decision without bytes or state mutation when ACK frame encoding fails, and keeps local and peer ACK directions independent | `settings_ack_state_starts_empty`, `settings_ack_accepts_local_batches_fifo`, `settings_ack_unexpected_failure_preserves_context`, `settings_ack_keeps_local_and_peer_ack_state_independent`, `settings_ack_peer_ack_coalesces_and_no_pending_preserves_state`, `settings_ack_peer_ack_send_emits_exact_bytes_and_clears_only_pending`, `settings_ack_peer_ack_encode_failure_preserves_state_reason_and_output`, `http2-core-settings-ack-state`, retained `http2-protocol-core-settings-unexpected-ack-human`, retained `http2-protocol-core-settings-unexpected-ack-json`, and retained aggregate output-chunk assertions in `http2-protocol-core` | pure local queue, peer ACK intent, failure context, no-pending, coalescing, exact emitted bytes, encode-failure output, and immutable input coverage moved to `core_test.veln`; focused external-package result and output-chunk evidence moved to `http2-core-settings-ack-state`; the monolithic case calls `std::http2::core` for acknowledgement bookkeeping while retaining frame decode, peer SETTINGS application, diagnostic projection, complete stdout, and integration ordering |
| local SETTINGS send helpers and `http2-local-settings`, `http2-local-settings-batch`, `http2-local-settings-ordered-batch`, local item boundary, policy-provenance, and no-output failure assertions | the public core facade owns the complete bounded local SETTINGS send transition for `SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_ENABLE_PUSH`, `SETTINGS_MAX_CONCURRENT_STREAMS`, `SETTINGS_INITIAL_WINDOW_SIZE`, `SETTINGS_MAX_FRAME_SIZE`, `SETTINGS_MAX_HEADER_LIST_SIZE`, and `SETTINGS_ENABLE_CONNECT_PROTOCOL`; accepted batches preserve caller order, emit exact bytes, record one outstanding ACK batch, and update local ENABLE_PUSH and ENABLE_CONNECT_PROTOCOL policy offsets; failures preserve input policy and ACK state and expose no bytes | `local_settings_send_accepts_every_supported_setting_boundary`, `local_settings_send_rejects_every_supported_setting_outside_range`, `local_settings_send_preserves_order_policy_offsets_and_ack_fifo`, `local_settings_send_empty_batch_emits_frame_and_records_ack`, `local_settings_send_failure_preserves_policy_ack_state_and_output`, `local_settings_send_rejects_client_enable_connect_protocol_without_output`, `http2-core-local-settings-send`, and retained aggregate output-chunk assertions in `http2-protocol-core` | pure supported-setting, boundary, endpoint-role, exact-byte, ordered-batch, empty-batch ACK tracking, local-policy, ACK FIFO, and failure-output preservation moved to `core_test.veln`; focused external-package public result and output-chunk evidence moved to `http2-core-local-settings-send`; the monolithic case calls `std::http2::core` for local SETTINGS sends while retaining broader receive, header-validation, complete stdout, and integration ordering |
| peer SETTINGS item validation assertions for supported values, unknown identifiers, duplicates, role rules, failure context, and invalid initial SETTINGS value precedence | the public core facade owns pure complete-payload validation for supported peer SETTINGS identifiers; unknown items are ignored; known duplicates are inspected in wire order; value-range and endpoint-role failures expose exact absolute item offsets, six-octet previews, stable diagnostic ids, metadata, and provenance without applying partial state | `peer_settings_validation_accepts_every_supported_setting_boundary`, `peer_settings_validation_rejects_supported_out_of_range_values`, `peer_settings_validation_ignores_unknown_items_and_preserves_wire_order`, `peer_settings_validation_rejects_endpoint_role_rules`, `peer_settings_validation_failure_preserves_input_and_exact_context`, `http2-protocol-core-settings-value-human`, `http2-protocol-core-settings-value-json`, `http2-protocol-core-settings-enable-push-role-human`, `http2-protocol-core-settings-enable-push-role-json`, and retained integration assertions in `http2-protocol-core` | pure supported-setting, boundary, unknown-item, duplicate ordering, first-failure, diagnostic-input, endpoint-role, and immutable-input coverage moved to `core_test.veln`; focused human and JSON cases obtain public typed failures through `http2::core`; the monolithic case calls the public validator while retaining SETTINGS state application, frame decode, ACK scheduling, receive-limit updates, initial-gate precedence, complete stdout, and wider integration |
| peer SETTINGS state projection assertions for defaults, known item application, duplicate settings, unknown settings, partial items, and peer-created stream high-water | immutable peer advertised SETTINGS state exposes protocol defaults separately from absent advertised values, applies complete validated payload items in wire order, keeps the last duplicate value active, ignores unknown identifiers, preserves state for partial payloads, records absolute item offsets, and updates peer-created stream high-water through an explicit state transition | `peer_settings_state_starts_with_protocol_defaults`, `peer_settings_state_applies_known_items_and_ignores_unknown_items`, `peer_settings_state_uses_last_duplicate_value`, `peer_settings_state_preserves_enable_push_and_connect_offsets_independently`, `peer_settings_state_preserves_state_for_partial_payloads`, `peer_settings_state_records_peer_created_stream_high_water`, `http2-core-peer-settings-state`, and retained integration assertions in `http2-protocol-core` | pure defaults, known-item state, duplicate last-value behavior, unknown-item preservation, independent enable-push and connect-protocol offset preservation, partial-payload state preservation, offset projection, and high-water immutability moved to `core_test.veln`; focused external-package result projections moved to `http2-core-peer-settings-state`; the monolithic case still retains frame decode, ACK scheduling, receive-limit updates, initial-gate precedence, complete stdout, and wider integration |
| peer-created stream admission monotonicity, ignored trailers, known stream reuse, failure context, and failure-state preservation assertions | immutable peer stream admission state starts empty, records only new non-trailer HEADERS streams, ignores already tracked streams and non-HEADERS frames, accepts empty and higher stream ids without advancing the input state, records high-water stream ids monotonically, rejects non-increasing stream ids with exact previous high-water, endpoint role, active state, rule provenance, and preview, and exposes no next state on failure | `peer_stream_admission_records_only_new_non_trailer_headers`, `peer_stream_admission_record_stream_id_is_monotonic`, `peer_stream_admission_acceptance_keeps_caller_owned_high_water`, `peer_stream_admission_rejects_non_increasing_stream_ids_without_next_state`, `http2-core-peer-stream-admission`, and retained `http2-protocol-core` stream lifecycle, complete stdout, and output-chunk integration | pure high-water, trailer, known-stream, monotonic recording, empty acceptance, higher-id acceptance, failure-data, preview, and immutable-input coverage moved to `core_test.veln`; focused external-package result projections moved to `http2-core-peer-stream-admission`; the monolithic case still owns actual receive-flow-control stream collections, reset and closed-stream integration, diagnostic projection, complete stdout, and wider transition ordering |
| aggregate connection state defaults, stream collection ownership, and component replacement | standard-owned immutable aggregate connection state composes endpoint role, next offset, connection preface, initial peer SETTINGS gate, pending header block, production HPACK dynamic table, peer SETTINGS state, SETTINGS ACK state, peer stream admission, empty stream collection, connection receive credit, local SETTINGS policy, and lifecycle; component replacement returns a new aggregate without mutating the input state | `connection_state_starts_with_composed_server_defaults`, `connection_state_starts_with_client_role`, `connection_state_updates_are_immutable`, `connection_state_replaces_protocol_gates_and_closed_lifecycle_immutably`, `stream_collection_adds_replaces_and_finds_stream_entries_immutably`, `stream_collection_updates_lifecycle_credit_and_content_length_immutably`, `stream_collection_missing_updates_preserve_all_components`, `connection_state_preserves_stream_collection_for_missing_stream_updates`, `http2-core-connection-state`, and retained `http2-protocol-core` receive/send transition integration | pure aggregate defaults, role-specific initialization, production HPACK table ownership, stream collection state ownership, stream lifecycle label and reset projection, receive and send credit projection, missing-stream no-op preservation, content-length counter projection, lifecycle projection, protocol-gate replacement, active pending-header replacement, closed lifecycle projection, and immutable component replacement moved to `core_test.veln`; focused external-package projections moved to `http2-core-connection-state` and `http2-core-stream-collection`; the monolithic case still owns HPACK-carrying receive transitions, header validation, per-frame flow-control debit/refill transitions, outbound transitions, complete stdout, and output-chunk integration |
| stream lifecycle receive-admission predicate helpers | standard-owned stream lifecycle state distinguishes open, client-push-associated, reserved-by-peer, reserved-local, half-closed-local, half-closed-remote, closed, and reset states; pure projections report active stream status, receive-window ownership, open-stream projection, DATA, RST_STREAM, WINDOW_UPDATE, and PRIORITY admission, active-state labels, rejection-rule labels, and reset error codes | `stream_lifecycle_predicates_cover_reserved_and_closed_states`, `stream_lifecycle_rejection_context_matches_protocol_labels`, and `http2-core-stream-collection` | pure lifecycle labels, active status, receive-window ownership, open projection, per-frame admission predicates, diagnostic active-state labels, rejection-rule labels, and reset error-code projection moved to `core_test.veln`; focused external-package projections moved to `http2-core-stream-collection`; the monolithic case still owns applying those decisions inside receive dispatch, header validation, flow-control transitions, complete stdout, and output-chunk integration |
| inbound frame-kind admission over stream collections | standard-owned receive-dispatch admission accepts connection controls and unknown extension kinds, rejects known stream frames on stream zero with connection-control context, accepts HEADERS before stream lookup, applies DATA, RST_STREAM, WINDOW_UPDATE, PRIORITY, and unknown extension admission to current stream lifecycle, accepts idle PRIORITY, accepts PUSH_PROMISE only on client-push-associated streams, and preserves stream collection plus frame-header preview on failure | `stream_frame_admission_accepts_connection_controls_and_unknown_extension_kinds`, `stream_frame_admission_rejects_wrong_connection_and_idle_stream_kinds`, `stream_frame_admission_applies_lifecycle_specific_receive_rules`, `stream_frame_admission_preserves_collection_and_preview_on_failure`, `http2-core-stream-frame-admission`, and retained `http2-protocol-core` receive-dispatch integration | pure connection-frame, idle-stream, lifecycle-specific DATA, RST_STREAM, WINDOW_UPDATE, PRIORITY, PUSH_PROMISE, unknown extension, exact failure-context, immutable-stream-collection, and preview-preservation coverage moved to `core_test.veln`; focused external-package projections moved to `http2-core-stream-frame-admission`; the monolithic case still owns payload parsing, HPACK-carrying receive transitions, header validation, flow-control debit/refill, complete stdout, and output-chunk integration |
| DATA receive flow-control application over aggregate connection state | standard-owned DATA receive flow-control debits both aggregate connection receive credit and the target stream receive credit immutably; connection-window failure, stream-window failure, and missing-stream failure expose exact context and preview without returning a next state or mutating the input aggregate | `data_receive_flow_control_debits_connection_and_stream_immutably`, `data_receive_flow_control_failures_preserve_state_and_preview`, `http2-core-data-receive-flow-control`, and retained `http2-protocol-core` DATA integration | pure aggregate-state DATA debit, connection failure, stream failure, missing-stream failure, exact failure context, immutable input-state, and preview-preservation coverage moved to `core_test.veln`; focused external-package state and failure projections moved to `http2-core-data-receive-flow-control`; the monolithic case still owns DATA payload parsing, content-length integration, frame dispatch ordering, diagnostic projection, complete stdout, and output-chunk integration |
| flow-control numeric domain helper assertions | connection window credit, stream window credit, configured initial window size, and `WINDOW_UPDATE` increment domains expose their role-specific bounds through the public core facade; debit and refill helpers return immutable next-credit decisions or exact domain failures without changing input credit or increment values | `flow_control_domains_accept_boundaries`, `flow_control_domains_reject_out_of_range_values`, `flow_control_debit_and_refill_are_immutable`, `flow_control_failures_preserve_input_credit`, `http2-core-flow-control-domains` | pure domain construction, boundary rejection, negative stream credit, debit, refill, overflow failure data, and input preservation moved to `core_test.veln`; focused external-package result projections moved to `http2-core-flow-control-domains`; monolithic DATA, peer `SETTINGS_INITIAL_WINDOW_SIZE`, received and outbound `WINDOW_UPDATE`, complete stdout, and diagnostic integration remain while the wider state machine is migrated |
| `initial_dynamic_core_state`, `empty_dynamic_core_state` | empty capacity, size, and count | `dynamic_table_starts_empty` | success |
| `dynamic_core_header_entry_size`, `dynamic_core_insert_entry_state` | name octets plus value octets plus 32 and immutable insertion | `dynamic_table_inserts_newest_first_with_exact_octet_size` | success and input-state preservation |
| `dynamic_core_entry_at` and newest/older accounting projections | one-based newest-to-oldest lookup and unavailable indices | `dynamic_table_lookup_is_one_based_and_preserves_arbitrary_value_octets` | success and unavailable lookup |
| `dynamic_core_bounded_entry`, `dynamic_core_evict_for_table_size` | oldest-first eviction and oversize-entry clearing | `dynamic_table_evicts_oldest_entries_until_the_newest_prefix_fits`, `dynamic_table_oversize_insertion_clears_only_the_result_table` | success and input-state preservation |
| `dynamic_core_with_table_size` | shrinking evicts, growing retains, and zero capacity stays empty | `dynamic_table_capacity_changes_are_immutable_and_evict_when_shrinking`, `dynamic_table_zero_capacity_stays_empty_until_a_later_insertion` | success and input-state preservation |
| dynamic-table construction and capacity transition failures | negative capacities are rejected without changing the input | `dynamic_table_rejects_negative_capacities_without_changing_input` | failure-state preservation |
| production octet-value accounting projections | values retain non-visible octets through lookup | `dynamic_table_lookup_is_one_based_and_preserves_arbitrary_value_octets` | success |
| `dynamic_core_decode_indexed_at`, saturated indexed-integer paths, newest/older projections, and indexed failure assertions | static indices 1 through 61 and newest-first dynamic indices decode through the immutable table; multi-octet indices, zero, malformed or incomplete integers, unavailable entries, exact value octets, and unchanged state have focused outcomes | `indexed_header_decoder_resolves_every_static_entry`, `indexed_header_decoder_resolves_dynamic_entries_newest_first`, `indexed_header_decoder_accepts_multi_octet_dynamic_indices`, `indexed_header_decoder_returns_focused_integer_and_zero_failures`, `indexed_header_decoder_reports_unavailable_dynamic_entries_without_next_state`, `hpack-indexed-header-field` | success, failure, raw octets, and input-state or failure-output preservation |
| literal-with-indexing, literal-without-indexing, never-indexed, direct-name, dynamic-name continuation, raw-string, Huffman-string, and literal failure assertion families | all three literal representations decode direct, static, and newest-first dynamic names with multi-octet indices and lengths; raw and Huffman values preserve exact octets; only incremental indexing inserts; focused name-index, unavailable-name, string-length, raw-truncation, invalid-name, and Huffman failures expose no field or next table and preserve input state | `literal_header_decoder_supports_all_representations_and_table_transitions`, `literal_header_decoder_preserves_raw_and_huffman_octets`, `literal_header_decoder_resolves_dynamic_names_and_multi_octet_prefixes`, `literal_header_decoder_returns_focused_name_failures_without_changing_state`, `literal_header_decoder_returns_focused_value_failures_without_changing_state`, `hpack-literal-header-field` | success, failure, raw result values, and input-state or failure-output preservation |
| table-size update direct, saturated, continuation, peer-limit, malformed, incomplete, wrong-prefix, shrink, growth, and state projections | one `001xxxxx` update decodes through the five-bit integer codec, enforces the explicit peer maximum, and applies an immutable capacity transition; focused failures expose no next table and preserve the input | `table_size_update_decoder_accepts_boundary_and_multi_octet_capacities`, `table_size_update_decoder_shrinks_with_eviction_and_grows_with_retention`, `table_size_update_decoder_returns_focused_failures_without_changing_state`, `hpack-table-size-update` | success, failure, result values, and input-state or failure-output preservation |
| recursive production decode, field-order, non-visible value, every literal form, in-block insertion and cross-decode dynamic-reference, list boundaries, update-only and leading table-size-update blocks, misplaced-update, nested codec-failure, and failure-state assertions | a complete arbitrary finite block composes the production field codecs in wire order, preserves exact octets, applies immutable state transitions, restricts bounded updates to the leading sequence, retains existing failure families, and exposes no partial list or next table | `header_block_decoder_preserves_order_octets_and_dynamic_transitions`, `header_block_decoder_composes_every_literal_form_and_list_boundaries`, `header_block_decoder_reuses_next_table_and_accepts_update_only_blocks`, `header_block_decoder_accepts_leading_updates_and_rejects_misplaced_updates`, `header_block_decoder_preserves_nested_failures_and_input_state`, `hpack-header-block-decoding` | success, failure, raw result values, and input-state or failure-output preservation |
| indexed, literal, Huffman/raw selection, dynamic-name continuation, recursive ordered-list, active-capacity, and outbound failure-preservation assertion families | production encoding emits exact static and multi-octet dynamic indices and the complete literal-representation by name-source matrix, preserves arbitrary octets and order, reuses in-block immutable state, applies capacity eviction, round trips through the decoder, and returns every reachable focused failure without bytes or next state; defensive integer, string, and table failures are unreachable from valid public values | `header_encoder_emits_exact_static_and_multi_octet_dynamic_indices`, `literal_encoder_covers_representations_name_sources_and_string_policies`, `header_block_encoder_preserves_order_reuses_in_block_state_and_round_trips`, `header_block_encoder_handles_empty_lists_capacity_eviction_and_recursive_lists`, `header_encoders_return_focused_failures_without_output_or_next_state`, `hpack-header-block-encoding` | success, exact emitted bytes and complete facade projections, raw octets, decode-after-encode, failure, and input-list, input-table, or failure-output preservation |
| `hpack_static_huffman_symbol`, code table, and payload encode helpers | the complete static table encodes arbitrary octets and applies EOS-prefix padding | `huffman_codec_preserves_canonical_vectors`, `huffman_codec_round_trips_every_single_octet`, `huffman_encoder_uses_eos_prefix_padding_at_bit_boundaries` | success and emitted bytes |
| checked Huffman decode loops and non-visible label projections | decoding preserves exact arbitrary octets without a label compatibility API | `huffman_codec_round_trips_every_single_octet`, `huffman_codec_round_trips_recursive_multi_octet_input`, `huffman_codec_preserves_non_visible_octets`, `hpack-huffman-codec` | success and raw octets |
| malformed padding, EOS, and incomplete-code assertion families | failures expose no partial decoded value and distinguish representative invalid payloads | `huffman_decoder_rejects_eos_invalid_padding_and_truncated_codes`, `huffman_decode_failure_exposes_no_partial_output`, `hpack-huffman-codec` | failure and failure-output preservation |

Pure cases move next to their owning standard module as small
`Result<(), String>` tests. Observable command behavior remains in focused
specification cases. The existing focused human and JSON cases should be
reused before creating new directories.

The complete stdout string is retained until all of its lines are classified.
It is removed as one change after the matrix reaches zero unclassified lines;
it is not gradually weakened with broader substring matching.

## Implementation Order

Each step must leave the standard package and workspace buildable:

1. Complete HPACK state and codec behavior, then move pure HPACK assertions to
   adjacent tests.
2. Complete connection, stream, settings, flow-control, headers, and shutdown
   transitions, moving pure assertions after each responsibility lands.
3. Move remaining human, JSON, CLI parsing, and output-chunk assertions to
   focused specification cases.
4. Audit old symbols, local fixture types, `require_*` calls, stdout lines, and
   output chunks. Classify every residual before deleting anything.
5. Delete the monolithic case and promote the final implemented behavior to
   `../specification/http2.md`. Archive this proposal under
   `../reference/implemented-proposals/` and remove it from the proposal
   catalog.

Do not combine a large implementation move with deletion of its only existing
coverage. The source fixture remains available until the replacement tests
pass independently.

## Verification Gates

### Standard Package

- `bash scripts/agent-stdlib-test` runs the toolchain-owned standard project
  through the guarded runner and executes every lowered standard test body.
- Frame, HPACK, diagnostics, and core tests cover their responsibility without
  importing fixture modules.
- The toolchain-owned standard project checks all source files, including
  modules unreachable from the prelude.
- Private implementation modules cannot be imported from user packages.
- Bare former HTTP/2 and HPACK helpers remain unavailable.

### Protocol Semantics

- Frame boundaries, preface, SETTINGS, continuation, stream lifecycle,
  flow-control, GOAWAY, header validation, and every supported HPACK
  representation have focused success and failure coverage.
- Failed decode and send transitions prove state, input, and output
  preservation.
- Diagnostic ids, human rendering, and structured protocol details remain
  stable, while raw nested diagnostic values use the intended new ADT shape.

### Loader and Performance

- An ordinary project does not load or analyze HTTP/2 modules.
- Importing one HTTP/2 facade loads only its dependency closure.
- Record guarded-run elapsed time and peak memory for an ordinary project, one
  HTTP/2 import, all standard HTTP/2 tests, and the monolithic case before its
  deletion.
- Investigate timeouts, kills, allocation failures, or material regression as
  implementation defects rather than increasing the runner limit.

Use `scripts/agent-run` and `scripts/agent-test` for the broad, generated, and
workspace checks.

## Fixture Deletion Gate

The directory `../../examples/specification/run/http2-protocol-core/` is
deletable only when all of the following are true:

- the migration matrix has zero unclassified `require_*` calls, stdout lines,
  and output-chunk assertions;
- no current implementation, standard test, or focused specification case
  imports any file from the directory;
- no reusable HPACK, stream, connection, settings, flow-control, header, or
  shutdown implementation remains there;
- standard package tests and focused specification cases pass without the
  directory;
- the workspace test suite passes after the directory is removed;
- old public symbols and fixture-only public codec names have no unclassified
  residuals; and
- `../specification/http2.md` describes the resulting implemented API and
  routes to its executable evidence.

## Current Completion Review

The completion criteria remain unsatisfied. Do not create `prompts/STOP`.

### Newly Promoted Stream-Collection Slice

- `std::http2::core` now has a standard-owned immutable stream collection,
  stream-entry lifecycle state, and inbound frame-kind admission boundary over
  the stream collection. The collection records stream ids, lifecycle labels
  including client-push-associated, reserved-by-peer, reserved-local,
  half-closed, closed, and reset states with reset error codes, receive and
  send stream-window credits, and content-length expected and observed
  counters. Add, replace, lookup, and focused update helpers preserve the
  caller's input collection. Public lifecycle predicates now expose active
  status, receive-window ownership, open-stream projection, DATA, RST_STREAM,
  WINDOW_UPDATE, and PRIORITY admission, active-state labels, and
  rejection-rule labels. The admission boundary applies those predicates to
  incoming frame kinds while preserving the input collection and preview on
  failure.
- `CoreConnectionState` now composes the stream collection as part of the
  aggregate standard-owned state. The adjacent `core_test.veln` checks empty
  aggregate defaults, immutable aggregate replacement, stream add/replace,
  lookup, active counts, credit replacement, lifecycle replacement,
  content-length replacement, and missing-stream update preservation. It also
  checks the public stream lifecycle predicate matrix and rejection-label
  projections. The focused
  `http2-core-connection-state` case records the public stream-count and
  active-stream-count projections from an ordinary external package,
  `http2-core-stream-collection` records stream-entry, missing-update, and
  lifecycle predicate projections, and
  `http2-core-stream-frame-admission` records the public admission decision and
  failure projections.
- `std::http2::core` now also owns the immutable DATA receive flow-control
  debit over `CoreConnectionState`: accepted DATA frames debit both aggregate
  connection credit and the target stream receive credit, while
  connection-window failure, stream-window failure, and missing-stream failure
  expose focused context and preserve the input aggregate and preview. The
  adjacent `core_test.veln` checks the success and failure-state invariants,
  and the focused `http2-core-data-receive-flow-control` case records public
  state and failure projections.
- These slices move frame-kind admission and DATA credit debit, but they do
  not yet move payload parsing, header validation, WINDOW_UPDATE credit
  transitions, outbound send transitions, or GOAWAY drain decisions out of the
  monolithic case.

### Remaining Deletion-Gate Blockers

- `examples/specification/run/http2-protocol-core/` still contains
  `main.veln`, `hpack_fixture.veln`, `hpack_static.veln`,
  `hpack_dynamic_core.veln`, and `case.toml`. The monolithic source still owns
  HPACK-carrying receive transitions, frame dispatch, header and
  content-length validation, per-frame receive and send flow-control
  integration, outbound transitions, graceful shutdown integration, complete
  stdout, and output-chunk integration.
- Fixture HPACK state and compatibility routes remain in the aggregate case;
  completed header blocks still need to be converted to the public typed
  `std::http2::hpack` codec boundary before the fixture can be removed.
- The checked migration matrix is not empty, and the aggregate exact stdout
  and output-chunk assertions remain deletion blockers until every line and
  chunk table has focused replacement coverage.

### Required Continuation

Continue from the new public stream collection and DATA credit debit by moving
payload parsing integration, WINDOW_UPDATE credit application,
header/content-length state transitions, outbound send transitions, and GOAWAY
drain behavior behind `std::http2::core`. Keep each transition immutable and
preserve failure-state atomicity across the connection, stream collection,
HPACK table, pending continuation, flow-control credits, and output bytes.

When the gate is met, remove `main.veln`, `hpack_fixture.veln`,
`hpack_static.veln`, `hpack_dynamic_core.veln`, and the monolithic
`case.toml` together. The earlier `stream_domain.veln` duplicate is already
removed. A smaller example may keep the directory name only if it has a
focused observable purpose and no longer contains the broad fixture
implementation or complete stdout assertion.

## Non-Goals

- TLS, ALPN, socket listeners, or platform networking
- mutable or effectful connection ownership inside the pure core
- production throughput optimization unrelated to removing an observed
  regression
- compatibility aliases for the former bare HTTP/2 or `hpack_fixture_*` API
- weakening diagnostics or JSON assertions to make migration easier
- adding another fixture case solely to increase a count, list width, table
  update count, or stream sequence length

## Completion Criteria

- The fixture deletion gate is satisfied and the monolithic case is removed or
  reduced to a focused example.
- `std::http2::hpack` and `std::http2::core` own all reusable behavior formerly
  implemented by the fixture.
- Production HPACK encoding and decoding return focused typed failures without
  fixture fallback ids.
- Standard tests and focused specification cases preserve all classified
  semantics and observable output.
- Current specification and executable evidence are updated before this
  proposal is archived as implemented history.
