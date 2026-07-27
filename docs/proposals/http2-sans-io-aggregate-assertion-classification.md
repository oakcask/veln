# HTTP/2 Aggregate Assertion Classification

Status: proposed

This artifact classifies the assertions removed with the retired
`../../examples/specification/run/http2-protocol-core/` aggregate case. It is a
completion-gate artifact for
[http2-sans-io-protocol-core.md](http2-sans-io-protocol-core.md), not a current
behavior specification.

## Extraction

Source revision: parent of `f91a4e42`.

Validation commands:

```sh
git show f91a4e42^:examples/specification/run/http2-protocol-core/main.veln | rg -o "\brequire_[A-Za-z0-9_]+\b" | sort | uniq -c
git show f91a4e42^:examples/specification/run/http2-protocol-core/case.toml | rg -c "^\[\[output_chunk_list\]\]"
```

The helper command finds 65 helper definitions and 717 whole-name occurrences,
so 652 invocation sites require classification. The output-table command finds
315 tables. The exact stdout value contains 2,044 newline-terminated lines; the
stdout table below partitions those lines by the first emitted token prefix.

The tables intentionally classify by invariant-preserving buckets rather than
copying the retired fixture. A bucket is valid only when the retained evidence
preserves the endpoint role, starting state, diagnostic precedence, result
projection, emitted bytes, and caller-state atomicity relevant to that bucket.

## Helper Invocation Matrix

| Helper | Invocations | Retained evidence | Status |
| --- | ---: | --- | --- |
| `require_client_push_equal_int` | 27 | `http2-core-peer-stream-admission/`, `http2-core-stream-collection/`, `http2-core-receive-frame-dispatch/`, focused stream-id diagnostics | classified |
| `require_client_push_equal_string` | 7 | `http2-core-peer-stream-admission/`, `http2-core-stream-collection/`, `http2-core-receive-frame-dispatch/`, focused stream-id diagnostics | classified |
| `require_client_push_ordering_rejection_preserves_state` | 11 | `http2-core-peer-stream-admission/`, `http2-core-stream-collection/`, `http2-core-receive-frame-dispatch/`, focused stream-id diagnostics | classified |
| `require_client_stream_id` | 2 | `http2-core-peer-stream-admission/`, `http2-core-stream-collection/`, `http2-core-receive-frame-dispatch/`, focused stream-id diagnostics | classified |
| `require_closed_peer_stream_reuse_failure` | 1 | `http2-core-peer-stream-admission/`, `http2-core-stream-collection/`, `http2-core-receive-frame-dispatch/`, focused stream-id diagnostics | classified |
| `require_completed_header_list_decode` | 10 | `hpack-header-block-decoding/`, `hpack-header-block-encoding/`, `http2-core-receive-frame-dispatch/` | classified |
| `require_connection_stream_id` | 3 | `http2-core-peer-stream-admission/`, `http2-core-stream-collection/`, `http2-core-receive-frame-dispatch/`, focused stream-id diagnostics | classified |
| `require_equal_int` | 72 | `core_test.veln` aggregate assertions and focused HTTP/2 cases | classified |
| `require_equal_string` | 16 | `core_test.veln` aggregate assertions and focused HTTP/2 cases | classified |
| `require_extended_connect_negotiation_cases` | 1 | `http2-core-header-list-validation/`, content-length and header diagnostic focused cases | classified |
| `require_failure_with_id` | 31 | `core_test.veln` aggregate assertions and focused HTTP/2 cases | classified |
| `require_frame_decoded` | 62 | `http2-core-receive-frame-dispatch/`, `http2-core-receive-connection-boundary/` | classified |
| `require_head_response_coverage` | 1 | `http2-core-header-list-validation/`, content-length and header diagnostic focused cases | classified |
| `require_header_block_bytes` | 7 | `hpack-header-block-decoding/`, `hpack-header-block-encoding/`, `http2-core-receive-frame-dispatch/` | classified |
| `require_headers_padding_rejection_preserves_state` | 4 | `core_test.veln` aggregate assertions and focused HTTP/2 cases | classified |
| `require_hpack_bytes` | 11 | `hpack-header-block-decoding/`, `hpack-header-block-encoding/`, `http2-core-receive-frame-dispatch/` | classified |
| `require_hpack_dynamic_core_index_failure` | 1 | `hpack-header-block-decoding/`, `hpack-header-block-encoding/`, `http2-core-receive-frame-dispatch/` | classified |
| `require_hpack_fixture_failure` | 12 | `http2-sans-io-fixture-marker-classification.md`, focused `hpack-fixture-*` diagnostics | classified |
| `require_hpack_multiple_table_size_cases` | 1 | `hpack-header-block-decoding/`, `hpack-header-block-encoding/`, `http2-core-receive-frame-dispatch/` | classified |
| `require_hpack_state_dynamic_entry_count` | 5 | `hpack-header-block-decoding/`, `hpack-header-block-encoding/`, `http2-core-receive-frame-dispatch/` | classified |
| `require_hpack_state_table_size` | 18 | `hpack-header-block-decoding/`, `hpack-header-block-encoding/`, `http2-core-receive-frame-dispatch/` | classified |
| `require_hpack_states_equal` | 5 | `hpack-header-block-decoding/`, `hpack-header-block-encoding/`, `http2-core-receive-frame-dispatch/` | classified |
| `require_hpack_static_request_authority_validation` | 1 | `hpack-header-block-decoding/`, `hpack-header-block-encoding/`, `http2-core-receive-frame-dispatch/` | classified |
| `require_hpack_static_request_scheme_validation` | 1 | `hpack-header-block-decoding/`, `hpack-header-block-encoding/`, `http2-core-receive-frame-dispatch/` | classified |
| `require_hpack_transition_header` | 43 | `hpack-header-block-decoding/`, `hpack-header-block-encoding/`, `http2-core-receive-frame-dispatch/` | classified |
| `require_informational_response_coverage` | 1 | `http2-core-header-list-validation/`, content-length and header diagnostic focused cases | classified |
| `require_no_content_body_failure_context` | 6 | `http2-core-header-list-validation/`, content-length and header diagnostic focused cases | classified |
| `require_no_content_response_coverage` | 1 | `http2-core-header-list-validation/`, content-length and header diagnostic focused cases | classified |
| `require_outbound_credit_update_rejected` | 9 | `http2-core-outbound-data-flow/`, flow-control diagnostic focused cases | classified |
| `require_outbound_data_rejected` | 5 | `http2-core-outbound-data-flow/`, flow-control diagnostic focused cases | classified |
| `require_outbound_headers_connection_accepted` | 12 | `http2-core-outbound-headers/`, `hpack-header-block-encoding/`, outbound diagnostic focused cases | classified |
| `require_outbound_headers_connection_failure` | 11 | `http2-core-outbound-headers/`, `hpack-header-block-encoding/`, outbound diagnostic focused cases | classified |
| `require_outbound_headers_connection_state_preserved` | 8 | `http2-core-outbound-headers/`, `hpack-header-block-encoding/`, outbound diagnostic focused cases | classified |
| `require_outbound_headers_connection_transition` | 7 | `http2-core-outbound-headers/`, `hpack-header-block-encoding/`, outbound diagnostic focused cases | classified |
| `require_outbound_headers_from_list` | 16 | `http2-core-outbound-headers/`, `hpack-header-block-encoding/`, outbound diagnostic focused cases | classified |
| `require_outbound_headers_hpack_transition` | 3 | `http2-core-outbound-headers/`, `hpack-header-block-encoding/`, outbound diagnostic focused cases | classified |
| `require_outbound_hpack_encode_state` | 3 | `http2-core-outbound-headers/`, `hpack-header-block-encoding/`, outbound diagnostic focused cases | classified |
| `require_outbound_hpack_encode_transition` | 45 | `http2-core-outbound-headers/`, `hpack-header-block-encoding/`, outbound diagnostic focused cases | classified |
| `require_outbound_push_promise_accepted` | 7 | `http2-core-outbound-headers/`, `http2-core-receive-frame-dispatch/`, focused promised-stream diagnostics | classified |
| `require_outbound_push_promise_failure` | 10 | `http2-core-outbound-headers/`, `http2-core-receive-frame-dispatch/`, focused promised-stream diagnostics | classified |
| `require_outbound_push_promise_from_list` | 6 | `http2-core-outbound-headers/`, `http2-core-receive-frame-dispatch/`, focused promised-stream diagnostics | classified |
| `require_outbound_push_promise_hpack_connection_transition` | 5 | `http2-core-outbound-headers/`, `http2-core-receive-frame-dispatch/`, focused promised-stream diagnostics | classified |
| `require_outbound_push_promise_hpack_transition` | 7 | `http2-core-outbound-headers/`, `http2-core-receive-frame-dispatch/`, focused promised-stream diagnostics | classified |
| `require_outbound_push_promise_state_preserved` | 9 | `http2-core-outbound-headers/`, `http2-core-receive-frame-dispatch/`, focused promised-stream diagnostics | classified |
| `require_outbound_settings_accepted` | 1 | `http2-core-local-settings-send/` | classified |
| `require_outbound_settings_rejected_for_role` | 1 | `http2-core-local-settings-send/` | classified |
| `require_peer_stream_failure_preserves_connection_state` | 13 | `http2-core-peer-stream-admission/`, `http2-core-stream-collection/`, `http2-core-receive-frame-dispatch/`, focused stream-id diagnostics | classified |
| `require_peer_stream_id` | 6 | `http2-core-peer-stream-admission/`, `http2-core-stream-collection/`, `http2-core-receive-frame-dispatch/`, focused stream-id diagnostics | classified |
| `require_production_hpack_state` | 9 | `hpack-header-block-decoding/`, `hpack-header-block-encoding/`, `http2-core-receive-frame-dispatch/` | classified |
| `require_production_inbound_failure` | 3 | `http2-core-receive-frame-dispatch/`, `http2-core-receive-connection-boundary/` | classified |
| `require_production_inbound_field` | 8 | `http2-core-receive-frame-dispatch/`, `http2-core-receive-connection-boundary/` | classified |
| `require_production_inbound_octets` | 10 | `http2-core-receive-frame-dispatch/`, `http2-core-receive-connection-boundary/` | classified |
| `require_production_inbound_transition` | 14 | `http2-core-receive-frame-dispatch/`, `http2-core-receive-connection-boundary/` | classified |
| `require_production_request_content_length_validation` | 1 | `http2-core-header-list-validation/`, content-length and header diagnostic focused cases | classified |
| `require_production_route_failure` | 5 | `http2-core-receive-frame-dispatch/`, `http2-core-receive-connection-boundary/` | classified |
| `require_production_state_octets` | 3 | `core_test.veln` aggregate assertions and focused HTTP/2 cases | classified |
| `require_production_update_route` | 2 | `http2-core-receive-frame-dispatch/`, `http2-core-receive-connection-boundary/` | classified |
| `require_projected_hpack_fixture_failure` | 6 | `http2-sans-io-fixture-marker-classification.md`, focused `hpack-fixture-*` diagnostics | classified |
| `require_request_header_failure_detail` | 11 | `http2-core-header-list-validation/`, content-length and header diagnostic focused cases | classified |
| `require_request_header_failure_fact` | 21 | `http2-core-header-list-validation/`, content-length and header diagnostic focused cases | classified |
| `require_response_header_failure_context` | 2 | `http2-core-header-list-validation/`, content-length and header diagnostic focused cases | classified |
| `require_response_header_failure_fact` | 1 | `http2-core-header-list-validation/`, content-length and header diagnostic focused cases | classified |
| `require_server_promised_stream_id` | 1 | `http2-core-outbound-headers/`, `http2-core-receive-frame-dispatch/`, focused promised-stream diagnostics | classified |
| `require_server_stream_id` | 1 | `http2-core-peer-stream-admission/`, `http2-core-stream-collection/`, `http2-core-receive-frame-dispatch/`, focused stream-id diagnostics | classified |
| `require_true` | 9 | `core_test.veln` aggregate assertions and focused HTTP/2 cases | classified |

Helper invocation total: 652. Unclassified helper invocations: 0.

## Exact Stdout Line Matrix

| First token prefix | Lines | Retained evidence | Status |
| --- | ---: | --- | --- |
| `hpack` | 445 | HPACK focused cases, production HPACK core tests, receive/send HPACK integration cases | classified |
| `output_chunk_list` | 339 | Output-table classification below plus focused emitted-byte cases | classified |
| `outbound` | 308 | standard send APIs and focused `http2-core-*` output cases | classified |
| `output_chunk` | 301 | Output-table classification below plus focused emitted-byte cases | classified |
| `peer` | 151 | stream admission, receive connection boundary, PUSH_PROMISE and stream-id focused diagnostics | classified |
| `client` | 76 | stream admission, receive connection boundary, PUSH_PROMISE and stream-id focused diagnostics | classified |
| `local` | 70 | standard send APIs and focused `http2-core-*` output cases | classified |
| `request` | 52 | header-list validation, content-length, lifecycle, and receive-dispatch focused cases | classified |
| `response` | 51 | header-list validation, content-length, lifecycle, and receive-dispatch focused cases | classified |
| `goaway` | 42 | receive shutdown, send GOAWAY, ordered receive boundary, and GOAWAY diagnostics | classified |
| `settings` | 38 | standard send APIs and focused `http2-core-*` output cases | classified |
| `headers` | 26 | header-list validation, content-length, lifecycle, and receive-dispatch focused cases | classified |
| `trailers` | 24 | header-list validation, content-length, lifecycle, and receive-dispatch focused cases | classified |
| `priority` | 22 | stream admission, receive connection boundary, PUSH_PROMISE and stream-id focused diagnostics | classified |
| `data` | 13 | DATA receive, outbound DATA, WINDOW_UPDATE flow-control cases | classified |
| `initial` | 12 | stream admission, receive connection boundary, PUSH_PROMISE and stream-id focused diagnostics | classified |
| `half` | 10 | header-list validation, content-length, lifecycle, and receive-dispatch focused cases | classified |
| `window` | 9 | DATA receive, outbound DATA, WINDOW_UPDATE flow-control cases | classified |
| `continuation` | 8 | frame decoding, preface, ordered receive, and protocol diagnostic focused cases | classified |
| `header` | 7 | header-list validation, content-length, lifecycle, and receive-dispatch focused cases | classified |
| `frame` | 6 | frame decoding, preface, ordered receive, and protocol diagnostic focused cases | classified |
| `rst` | 6 | standard send APIs and focused `http2-core-*` output cases | classified |
| `preface` | 5 | frame decoding, preface, ordered receive, and protocol diagnostic focused cases | classified |
| `ping` | 5 | standard send APIs and focused `http2-core-*` output cases | classified |
| `max` | 4 | frame decoding, preface, ordered receive, and protocol diagnostic focused cases | classified |
| `unknown` | 2 | frame decoding, preface, ordered receive, and protocol diagnostic focused cases | classified |
| `server` | 2 | stream admission, receive connection boundary, PUSH_PROMISE and stream-id focused diagnostics | classified |
| `enable` | 2 | frame decoding, preface, ordered receive, and protocol diagnostic focused cases | classified |
| `push` | 2 | stream admission, receive connection boundary, PUSH_PROMISE and stream-id focused diagnostics | classified |
| `valid` | 1 | frame decoding, preface, ordered receive, and protocol diagnostic focused cases | classified |
| `incomplete` | 1 | frame decoding, preface, ordered receive, and protocol diagnostic focused cases | classified |
| `closed` | 1 | frame decoding, preface, ordered receive, and protocol diagnostic focused cases | classified |
| `pending` | 1 | frame decoding, preface, ordered receive, and protocol diagnostic focused cases | classified |
| `invalid` | 1 | frame decoding, preface, ordered receive, and protocol diagnostic focused cases | classified |
| `http2` | 1 | frame decoding, preface, ordered receive, and protocol diagnostic focused cases | classified |

Stdout line total: 2,044. Unclassified stdout lines: 0.

## Output Table Matrix

| Assertion name prefix | Tables | Retained evidence | Status |
| --- | ---: | --- | --- |
| `http2-client-push-promise` | 5 | PUSH_PROMISE receive/send and promised-stream ordering cases | classified |
| `http2-continuation` | 1 | pending-header-block and CONTINUATION receive cases | classified |
| `http2-data` | 26 | DATA receive and outbound DATA focused cases | classified |
| `http2-goaway` | 10 | GOAWAY receive/send and shutdown focused cases | classified |
| `http2-header-list` | 1 | receive-dispatch and outbound header focused cases | classified |
| `http2-headers` | 49 | receive-dispatch and outbound header focused cases | classified |
| `http2-hpack` | 123 | HPACK decode/encode focused cases and production header receive/send cases | classified |
| `http2-request-headers` | 9 | receive-dispatch and outbound header focused cases | classified |
| `http2-response-headers` | 8 | receive-dispatch and outbound header focused cases | classified |
| `http2-settings` | 2 | initial peer settings, local settings, and settings ACK output cases | classified |
| `http2-trailers` | 4 | receive-dispatch and outbound header focused cases | classified |
| `other` | 77 | named control-send, receive-flow, local-settings, PING, PRIORITY, PUSH_PROMISE, unknown-extension, and frame-header focused cases listed below | classified |

The `other` bucket contains explicit assertion names for peer header-table-size
limits, half-closed local padded content, content-length DATA, closed peer DATA,
preface unknown preservation, unknown extension preservation, connection and
stream WINDOW_UPDATE, peer and local SETTINGS, RST_STREAM, PRIORITY,
PUSH_PROMISE, PING, outbound PING, and max stream-header encoding. Each has a
same-named or responsibility-named focused route under
`../../examples/specification/run/` or adjacent standard coverage in
`../../crates/veln-stdlib/veln/http2/core_test.veln`.

Output table total: 315. Unclassified output tables: 0.

## Completion Impact

The retired aggregate assertion matrix is classified to zero unclassified
entries. The focused fixture-marker inventory remains classified by
[http2-sans-io-fixture-marker-classification.md](http2-sans-io-fixture-marker-classification.md).

The active proposal may be archived only after stale historical routes are
reconciled and the guarded verification gates pass.
