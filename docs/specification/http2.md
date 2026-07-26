# HTTP/2 Standard Modules

HTTP/2 support is opt-in. Source files import the required public module from
the toolchain-owned `std` package; no HTTP/2 function is part of the implicit
prelude.

```veln
use http2::frame from "std"
use http2::diagnostic from "std"
use http2::hpack from "std"
use http2::hpack::diagnostic from "std"
use http2::core from "std"
```

The public routes are:

- `http2::frame`: frame decoding and validated frame-header encoding.
- `http2::diagnostic`: protocol and peer-limit diagnostic constructors.
- `http2::hpack`: prefixed-integer and HPACK Huffman codecs, static entries,
  immutable dynamic-table state, table-size updates, indexed and literal
  header-field encoding, and complete header-block encoding and decoding.
- `http2::hpack::diagnostic`: HPACK diagnostic constructors.
- `http2::core`: connection and role-specific stream-id domains, immutable
  connection-preface and initial-peer-SETTINGS transitions, pure frame
  payload-length validation, immutable pending header-block sequencing,
  immutable local SETTINGS send transitions, immutable SETTINGS
  acknowledgement state, and pure PING request and ACK response transitions.

Nested implementation modules below `http2::hpack` and `http2::core` are not
package exports.
The JVM adapter keeps its intrinsic link names private; source code calls only
the module-qualified API. Diagnostic ids, human rendering, and
`details.protocol_diagnostic` projections remain stable.

`http2::core::validate_frame_payload_length(...)` performs pure inbound frame
shape validation. The caller supplies the active-state label associated with
its protocol projection. The function returns either a typed success or a
`FramePayloadLengthFailure` containing the offset, frame kind, stream id,
observed and expected lengths, active-state label, rule provenance, and exact
supplied payload preview. Rejection exposes no partial success and does not
alter the caller's immutable preview.

PING is exactly 8 payload octets; GOAWAY is at least 8; WINDOW_UPDATE and
RST_STREAM are exactly 4; and PRIORITY is exactly 5. A SETTINGS frame with ACK
is empty, while a SETTINGS frame without ACK has a length divisible by 6.
HEADERS requires the PADDED and PRIORITY prefixes selected by its flags.
PUSH_PROMISE requires its promised-stream prefix and the optional PADDED
prefix. DATA, CONTINUATION, and unknown kinds have no additional fixed or
minimum length constraint in this validator. Padding content, frame-header
decoding, and maximum-frame-size validation remain separate responsibilities.

`http2::core::send_ping(payload)` accepts exactly eight opaque payload octets
and emits one complete frame byte chunk with a length-`8`, kind-`6`,
flags-`0`, stream-`0` header followed by the unchanged payload. Seven, nine,
or otherwise invalid payload lengths return the shared
`FramePayloadLengthFailure` from `validate_frame_payload_length(...)`, expose
no bytes, and leave the caller's payload unchanged. Encoding failure is a
distinct transition shape, although the public constants are representable.

`http2::core::respond_to_ping(flags, payload)` emits one ACK frame byte chunk
for a validated non-ACK PING by preserving the eight-octet payload and using a
length-`8`, kind-`6`, flags-`1`, stream-`0` header. A received PING ACK returns
an explicit no-response action with no bytes, preventing an ACK loop.

`http2::core::send_local_settings_batch(...)` accepts a caller-ordered batch
of the supported local SETTINGS items:
`SETTINGS_HEADER_TABLE_SIZE`, `SETTINGS_ENABLE_PUSH`,
`SETTINGS_MAX_CONCURRENT_STREAMS`, `SETTINGS_INITIAL_WINDOW_SIZE`,
`SETTINGS_MAX_FRAME_SIZE`, `SETTINGS_MAX_HEADER_LIST_SIZE`, and
`SETTINGS_ENABLE_CONNECT_PROTOCOL`. Accepted batches emit exactly one
length-`6 * item_count`, kind-`4`, flags-`0`, stream-`0` SETTINGS frame,
preserve item order in the payload, update the local sent policy for
`SETTINGS_ENABLE_PUSH` and `SETTINGS_ENABLE_CONNECT_PROTOCOL` with the
six-byte item offset, and record exactly one outstanding local SETTINGS batch
in the caller-supplied `SettingsAckState`.

Local `SETTINGS_ENABLE_PUSH` and `SETTINGS_ENABLE_CONNECT_PROTOCOL` accept
only `0..1`; `SETTINGS_INITIAL_WINDOW_SIZE` accepts `0..2147483647`;
`SETTINGS_MAX_FRAME_SIZE` accepts `16384..16777215`; and the remaining
four-byte SETTINGS values accept `0..4294967295`. A client endpoint cannot
send `SETTINGS_ENABLE_CONNECT_PROTOCOL`. Validation and encoding failures
return a typed decision with no output bytes and without exposing a next
policy or next ACK state.

`http2::core::empty_settings_ack_state()` creates immutable state with no
outstanding local SETTINGS batch and no pending peer SETTINGS ACK. Local
SETTINGS senders record each already validated and emitted batch through
`record_local_settings_batch(...)`, which keeps the first setting identifier
and item count for FIFO acknowledgement projections. `accept_settings_ack(...)`
accepts a validated peer SETTINGS ACK only when a local batch is outstanding;
it removes exactly the oldest batch. Without an outstanding batch, it returns
`SettingsAckFailure` with the stable
`http2.protocol.unexpected_settings_ack` id, offset, active-state label, rule
provenance, and caller-supplied preview without exposing a next state.

`settings_ack_after_peer_frame(...)` records one pending outbound ACK after a
validated non-ACK peer SETTINGS frame with payload items and coalesces later
peer SETTINGS frames into the same pending intent. `send_pending_settings_ack`
returns a no-pending action with no bytes when there is no intent; otherwise
it emits exactly `000000040100000000` and clears only the peer-ACK side of the
state. The local outstanding queue, peer advertised SETTINGS values, HPACK
state, flow-control state, stream state, and shutdown state remain caller-owned
and separate from this acknowledgement state.

The adjacent
[`core_test.veln`](../../crates/veln-stdlib/veln/http2/core_test.veln) checks
one-below, exact, and one-above boundaries where distinct, the complete
HEADERS and PUSH_PROMISE flag matrix, unconstrained kinds, exact failure data,
preview preservation, and absence of success output on rejection. Focused
payload-length human and JSON cases project the active-state label and the
failure's stored preview through
`http2::diagnostic`; the aggregate protocol-core case retains wider decode
ordering and complete-stdout integration.
The same adjacent test checks exact PING request bytes, seven- and nine-octet
rejection, failure preview and output preservation, exact ACK bytes, payload
preservation, and received-ACK no-response behavior. The focused
[`http2-core-ping-transitions`](../../examples/specification/run/http2-core-ping-transitions/)
case imports `http2::core` from `std` and records the public request, ACK,
no-response, representative failure projections, and emitted bytes.
The adjacent test also checks SETTINGS ACK FIFO acknowledgement, unexpected ACK
failure context, independent local and peer ACK state, peer ACK coalescing,
no-pending behavior, exact emitted bytes, encode-failure output preservation,
and failure/input preservation. The focused
[`http2-core-settings-ack-state`](../../examples/specification/run/http2-core-settings-ack-state/)
case imports `http2::core` from `std` and records public success, no-pending,
representative failure, FIFO, coalescing, and exact-byte projections.
The same adjacent test checks every supported local SETTINGS item as one
bounded set, exact accepted bytes, ordered multi-item batches, local policy
offsets, ACK queue integration, endpoint role rejection, and immutable
failure/output behavior. The focused
[`http2-core-local-settings-send`](../../examples/specification/run/http2-core-local-settings-send/)
case imports `http2::core` from `std` and records public result and
output-chunk projections for accepted and rejected local SETTINGS sends.

`http2::core::empty_connection_preface(starting_offset)` creates immutable
state for the 24-octet client connection preface.
`accept_connection_preface(state, input)` accepts a complete preface in one
chunk or retains its matched prefix across arbitrary chunk boundaries. A
successful transition reports completion and exposes every trailing input
octet through `connection_preface_suffix(...)` for the later initial-SETTINGS
transition.

A mismatch failure reports its absolute offset, expected and actual octets,
matched count, and preview input. `close_connection_preface(state)` reports a
distinct partial-preface failure with the original starting offset, pending
count, and preview. Failures expose neither a next state nor a consumable
suffix, and the input state remains unchanged. The adjacent
[`core_test.veln`](../../crates/veln-stdlib/veln/http2/core_test.veln) checks
complete, byte-by-byte, unevenly chunked, trailing-input, first, middle, and
final mismatch, partial-close, and immutable failure-state behavior. The
focused `http2-protocol-core-preface-{invalid,partial}-{human,json}` cases
obtain these public failures and project them through `http2::diagnostic`;
the aggregate
[`http2-protocol-core`](../../examples/specification/run/http2-protocol-core/)
case retains initial-SETTINGS integration and its complete stdout assertion.

`http2::core::server_initial_peer_settings_gate()` and
`client_initial_peer_settings_gate()` create immutable role-specific state for
the first complete peer frame. `accept_initial_peer_settings(...)` accepts
only a non-ACK SETTINGS frame. A successful transition exposes the accepted
next state and retains the endpoint role.

A non-SETTINGS frame or initial SETTINGS ACK returns an
`InitialPeerSettingsFailure` with the stable diagnostic id, offset, frame kind,
flags, stream id, endpoint role, active-state label, rule provenance, and exact
supplied frame-header preview. Rejection exposes no transition or next state
and leaves the input state and preview unchanged. Frame-header completeness,
maximum-frame-size validation, stream-id and SETTINGS payload validation, and
SETTINGS value application remain separate transition stages.

The adjacent
[`core_test.veln`](../../crates/veln-stdlib/veln/http2/core_test.veln) checks
accepted server and client roles, acceptance without mutating the input state
or preview, non-SETTINGS and ACK rejection, exact failure context, and
immutable failure state and preview preservation. The focused
`http2-initial-peer-settings-gate-{human,json}` cases obtain the typed public
failure and project it through `http2::diagnostic`. The aggregate
[`http2-protocol-core`](../../examples/specification/run/http2-protocol-core/)
case calls the same public gate after complete-header, frame-size, and
complete-frame checks, then retains wider stream-id, payload, SETTINGS-value,
state, and complete-stdout integration coverage.

`http2::core::empty_pending_header_block()` constructs idle continuation
state. `start_header_block(...)` accepts an already validated HEADERS or
PUSH_PROMISE fragment. END_HEADERS completes the block immediately; otherwise
the returned immutable state retains the initiating stream, frame kind,
offset, flags, trailer classification, promised stream id, and accumulated
octets.

`continue_header_block(...)` accepts only CONTINUATION on the initiating
stream, appends fragments in wire order, and exposes a completed block only
after END_HEADERS. Completion preserves END_STREAM and trailer status from
HEADERS or the promised stream id from PUSH_PROMISE. Non-final transitions
expose no completed block. `close_pending_header_block(...)` accepts idle
input and rejects closure while a block remains active.

Typed failures distinguish a different frame kind, a different stream, and
closed input. They expose the current offset and frame coordinates, initiating
coordinates, expected stream, accumulated byte count, rule provenance, and
preview octets without exposing a next state or completed block. The input
state remains unchanged. Adjacent
[`core_test.veln`](../../crates/veln-stdlib/veln/http2/core_test.veln) coverage
checks immediate, multi-frame HEADERS, multi-frame PUSH_PROMISE, non-final,
wrong-kind, wrong-stream, active and idle closed-input paths, and exact
diagnostic-input preservation. The focused
`http2-protocol-core-continuation-*` cases project the public failures through
the stable human and JSON diagnostics, while the aggregate
[`http2-protocol-core`](../../examples/specification/run/http2-protocol-core/)
case retains frame decoding, HPACK decoding, stream-lifecycle, and output
integration coverage.

`http2::hpack::encode_integer(value, prefix_bits, representation_bits)` accepts
a non-negative `Int` and a prefix width from one through eight. It preserves
the caller-supplied high representation bits in the first octet and returns
the finite HPACK continuation encoding as a `ByteChunk`.
`http2::hpack::decode_integer(input, prefix_bits)` uses the same width contract
and reports the decoded value plus the consumed octet count. Empty input,
invalid widths, incomplete continuations, and encodings beyond the `Int` range
are rejected. The canonical multi-octet encoding and representation-bit
behavior are checked by
`../../examples/specification/run/hpack-prefixed-integer-codec/`.

`http2::hpack::encode_huffman(bytes)` encodes arbitrary `ByteChunk` octets
with the HPACK static Huffman table and the required EOS-prefix padding.
`decode_huffman(input)` returns the exact decoded `ByteChunk`, including
non-visible octets. It rejects EOS as a payload symbol, invalid or overlong
padding, and truncated or invalid code sequences. A failure returns no partial
decoded value. The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks canonical vectors, every single octet, recursive input, padding
boundaries, and rejection paths. The focused
[`hpack-huffman-codec`](../../examples/specification/run/hpack-huffman-codec/)
case records the public facade's encoded and decoded octets and representative
failures.

`http2::hpack::static_entry(index)` exposes every one-based HPACK static-table
entry from 1 through 61; `static_entry_name(entry)` and
`static_entry_value(entry)` project its exact fields.
`static_entry_index(name, value)` returns the exact entry index, while
`static_name_index(name)` returns the first index with the exact name. Indices
outside the table and unknown names or values return `None`. The complete
forward and reverse contract is checked by the adjacent standard-library
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln).

`http2::hpack::empty_dynamic_table(capacity)` creates an immutable empty table
and rejects a negative capacity. `insert_dynamic_table_entry(table, name,
value)` inserts at index one, keeps entries in newest-to-oldest order, and
accounts for each entry as the name octet count plus the value octet count plus
32. It evicts the oldest entries until the result fits; an entry larger than
the active capacity clears the result table. Header values are `ByteChunk`
values and preserve arbitrary octets.

`dynamic_table_with_capacity(table, capacity)` returns a new table, evicting
after a shrink and retaining entries after a grow, and rejects a negative
capacity. Successful and failed transitions leave the input table unchanged.
The capacity, current size, and entry count have dedicated projections.
`dynamic_table_entry(table, index)` performs one-based lookup and returns
`None` for non-positive or unavailable indices; `dynamic_entry_name` and
`dynamic_entry_value` project a found entry. The adjacent standard-library
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks exact size accounting, insertion order, eviction, capacity changes,
lookup boundaries, arbitrary-octet values, and input-state preservation. The
focused
[`hpack-dynamic-table-state`](../../examples/specification/run/hpack-dynamic-table-state/)
case checks the same facade from an external package and records its projected
state and octet values through command output.

`header_field(name, value)`, `empty_header_list()`, and
`prepend_header_field(header, remaining)` construct an ordered encode input
while preserving every value as an exact `ByteChunk`.
`encode_indexed_header_field(header, index, table)` validates that the selected
static or newest-first dynamic entry exactly matches the field before emitting
the full seven-bit-prefixed indexed representation.
`encode_literal_header_field(header, representation, name_index,
huffman_name, huffman_value, table)` emits one explicitly selected literal.
Representation `0` means incremental indexing, `1` means without indexing, and
`2` means never indexed. Name index zero emits the direct name; other indices
must resolve to the field's exact static or dynamic name. The two Boolean
selectors independently choose raw or HPACK Huffman encoding for a direct name
and the value.

`encode_header_block(headers, table, active_capacity)` recursively encodes any
finite `HeaderList` in order. Its deterministic policy uses an exact static
entry first, then an exact dynamic entry; otherwise it emits an
incrementally-indexed literal with a static name, dynamic name, or direct name
in that order. Each string uses Huffman only when the complete Huffman literal
is shorter than its raw literal, so ties remain raw. A successful insertion is
available to later fields in the same block. When the supplied table capacity
exceeds `active_capacity`, the block starts with the required table-size update
and applies immutable oldest-first eviction before encoding fields.

Header encode transitions expose only complete bytes and the next immutable
table. Typed failures distinguish invalid representations or names, zero and
unavailable indices, indexed-field mismatches, integer or string encoding, and
table transitions. Block failures add the zero-based field position and active
capacity selection. A failure exposes no partial bytes or next state and
leaves the input list and table unchanged. Invalid representations and names,
zero indices, unavailable static and dynamic indices, indexed-field
mismatches, invalid active capacity, and nested field failures are reachable
through public encoder calls. The integer, string, and table failure variants
are defensive mappings for private codec failures that valid public values
cannot produce. The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks exact static and multi-octet dynamic bytes, the complete literal-form by
name-source matrix, raw and Huffman strings, empty and non-visible values,
in-block reuse, capacity eviction, list boundaries, decode-after-encode
behavior, every reachable failure, and input preservation. The focused
[`hpack-header-block-encoding`](../../examples/specification/run/hpack-header-block-encoding/)
case records public encoded bytes, ordered decoded values, next-state
projections, and representative typed failures.

`http2::hpack::decode_table_size_update(input, table, peer_maximum)` decodes
one `001xxxxx` dynamic table-size update with the five-bit-prefixed integer
codec. The transition reports the requested capacity, consumed octet count,
and next immutable table. Shrinking evicts oldest entries through the ordinary
capacity transition, while growing retains entries.

The typed `TableSizeUpdateFailure` distinguishes a different representation,
a malformed integer, an incomplete integer, and a capacity above the explicit
peer-advertised maximum. Capacity-limit projections report both the requested
capacity and the peer maximum. Every failure contains no next table and leaves
the input table unchanged. The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks boundary and multi-octet values, shrink and growth transitions, every
failure class, and state preservation. The focused
[`hpack-table-size-update`](../../examples/specification/run/hpack-table-size-update/)
case records public transition values and representative typed failures.

`http2::hpack::decode_indexed_header_field(input, table)` decodes one HPACK
indexed header-field representation with the full seven-bit-prefixed integer.
Indices 1 through 61 resolve through the static table. Larger indices resolve
through the supplied immutable dynamic table, where index 62 selects its
newest entry. The transition reports the consumed octet count, decoded name
and exact value `ByteChunk`, and the unchanged dynamic table.

The typed `IndexedDecodeFailure` distinguishes malformed and incomplete
integers, index zero, unavailable static entries, and unavailable dynamic
entries. Failure projections expose a stable failure kind and the requested
table coordinates where applicable; a failure contains neither a decoded
header nor a next table. The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks every static entry, single- and multi-octet dynamic indices, arbitrary
value octets, all reachable failure classes, and state preservation. The
focused
[`hpack-indexed-header-field`](../../examples/specification/run/hpack-indexed-header-field/)
case checks the public facade from an external package.

`http2::hpack::decode_literal_header_field(input, table)` decodes one literal
header field with incremental indexing, without indexing, or marked never
indexed. A zero name index reads a raw or Huffman string name; a nonzero name
index resolves through the static or immutable dynamic table with the
representation's full prefixed integer. The value is another raw or Huffman
string and is returned as an exact `ByteChunk`, including non-visible octets.
The transition identifies the representation and reports its decoded field,
consumed octet count, and next table. Incremental indexing inserts the field
into the next table; the other representations return the unchanged table.

The typed `LiteralDecodeFailure` distinguishes malformed and incomplete name
indices, unavailable indexed names, malformed and incomplete name or value
lengths, truncated raw name or value octets, invalid name octets, and name or
value Huffman failures. Truncation projections report the expected and
available octet counts, while unavailable dynamic-name projections report the
requested table coordinates. Every failure contains neither a decoded field
nor a next table and leaves the input table unchanged. The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks all representations, direct and indexed names, raw and Huffman strings,
multi-octet indices and lengths, exact value octets, insertion, and failure
preservation. The focused
[`hpack-literal-header-field`](../../examples/specification/run/hpack-literal-header-field/)
case records public raw result values and representative typed failures.

`http2::hpack::decode_header_block(input, table, peer_maximum)` recursively
decodes a complete ordered block of indexed and literal fields. `HeaderList`
keeps wire order, and every `HeaderField` retains its value as an exact
`ByteChunk`. The transition reports the full list, total consumed octets, and
next immutable table, so an incrementally indexed field is available to later
fields in the same block and to a later decode.

One or more bounded table-size updates may lead the block. An update after the
first field is a focused misplaced-update failure. Indexed, literal, and
table-size-update codec failures remain available as their existing typed
families beneath the block failure. A failure exposes neither a partial list
nor a next table and leaves the caller's input table unchanged. The adjacent
[`hpack_test.veln`](../../crates/veln-stdlib/veln/http2/hpack_test.veln)
checks empty and mixed blocks, field order, exact non-visible octets, dynamic
transitions across fields and decodes, every literal representation, list
boundaries, update-only and leading-update blocks, nested failures, and
failure-state preservation.
The focused
[`hpack-header-block-decoding`](../../examples/specification/run/hpack-header-block-decoding/)
case records public result values and representative failure kinds.

Additional executable evidence lives in the adjacent standard-library
`*_test.veln` files and in the focused HTTP/2 cases under
`../../examples/specification/`.
The broad protocol-core case remains coverage for state transitions and output
chunk projections while focused cases retain human and JSON diagnostic
coverage.
