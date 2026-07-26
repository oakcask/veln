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
- frame-kind, payload-length, stream-id-domain, and continuation sequencing;
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

The immutable dynamic-table slice has this checked migration matrix. These
rows migrate the pure state invariants from `hpack_dynamic_core.veln` to the
adjacent standard-library `hpack_test.veln`; existing fixture codec and output
projections remain because they also protect representation integration and
observable output.

| Source helper or assertion family | Migrated invariant | Destination | Coverage |
| --- | --- | --- | --- |
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
| indexed, literal, Huffman/raw selection, dynamic-name continuation, recursive ordered-list, active-capacity, and outbound failure-preservation assertion families | production encoding emits exact static and multi-octet dynamic indices and all literal forms, preserves arbitrary octets and order, reuses in-block immutable state, applies capacity eviction, round trips through the decoder, and returns focused failures without bytes or next state | `header_encoder_emits_exact_static_and_multi_octet_dynamic_indices`, `literal_encoder_covers_representations_name_sources_and_string_policies`, `header_block_encoder_preserves_order_reuses_in_block_state_and_round_trips`, `header_block_encoder_handles_empty_lists_capacity_eviction_and_recursive_lists`, `header_encoders_return_focused_failures_without_output_or_next_state`, `hpack-header-block-encoding` | success, exact emitted bytes, raw octets, decode-after-encode, failure, and input-state or failure-output preservation |
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

When the gate is met, remove `main.veln`, `stream_domain.veln`,
`hpack_fixture.veln`, `hpack_static.veln`, `hpack_dynamic_core.veln`, and the
monolithic `case.toml` together. A smaller example may keep the directory name
only if it has a focused observable purpose and no longer contains the broad
fixture implementation or complete stdout assertion.

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
