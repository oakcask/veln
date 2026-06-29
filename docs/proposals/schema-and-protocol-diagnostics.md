# Schema And Protocol Diagnostics

Status: proposed

This proposal defines diagnostics for schema, codec, and protocol-state
failures. It is a prerequisite for the HTTP/2 binary schema design driver
because byte-level failures must be repairable by agents and distinguishable
from stream-state protocol errors.

Implemented behavior for closed-input fixed-width `ByteView` read truncation,
including `codec.incomplete_input` byte offset, field path, byte counts, and
readiness details in `run --json` and stable human `run` output, is specified
under `../specification/run-json.md` and `../specification/commands.md` and
checked by
`../../examples/specification/run/binary-byteview-read-failure-json/` and
`../../examples/specification/run/binary-byteview-read-failure-human/`. The
implemented first schema-owned fixed-field mismatch slice, including
`schema.fixed_field_mismatch`, byte offset, field path, expected and actual
values, structured byte preview fields in `run --json`, and stable human
`run` bounded nearby-byte preview output, is specified under
`../specification/run-json.md`, `../specification/commands.md`, and
`../specification/execution.md` and checked by
`../../examples/specification/run/binary-fixed-field-mismatch-json/` and
`../../examples/specification/run/binary-fixed-field-mismatch-human/` for
direct byte helpers and by
`../../examples/specification/run/binary-schema-fixed-field-mismatch-json/`
and
`../../examples/specification/run/binary-schema-fixed-field-mismatch-human/`
for generated binary schema decode helpers. The
implemented frame-header schema truncation, reserved-bit mismatch, payload
length boundary, and field-local validation slices are specified under
`../specification/run-json.md` and `../specification/commands.md` and checked
by the binary schema frame-header, frame-payload, and validation cases under
`../../examples/specification/run/`; their JSON details use structured byte
preview fields while human output keeps bounded nearby-byte preview notes. The
implemented HTTP/2 protocol-state projections use
`http2.protocol.partial_preface` and `http2.protocol.invalid_preface` for
client connection preface failures,
`http2.peer_limit.frame_size_exceeded` for frame-size peer-limit failures and
`http2.protocol.invalid_frame_kind` for invalid connection-state and
stream-state frame kinds. Stream id domain failures use
`http2.protocol.invalid_stream_id` with frame kind, stream id, required stream
id domain, endpoint role, active state, and rule provenance. The fixed
payload-length projection uses
`http2.protocol.invalid_payload_length` for SETTINGS ACK, PING, GOAWAY,
`RST_STREAM`, and `WINDOW_UPDATE` payload-length failures, including
protocol-owned inspected-payload byte previews. Post-GOAWAY peer-created
stream failures use `http2.protocol.stream_after_goaway` with
attempted stream id, recorded last stream id, shutdown state, endpoint role,
active state, and rule provenance. These protocol projections are checked by
the HTTP/2 protocol-core cases under
`../../examples/specification/run/`. Generated exact-width binary schema
encode range failures use `codec.encode_value_unrepresentable` through
direct `byte_encode_<schema>` helpers, derived codec encode calls, and the
HTTP/2 frame-header encode fixture, including selected dispatch payload encode
paths that surface the same generated helper failure; the current behavior is
specified under `../specification/execution.md` and checked by the generated
encode cases under `../../examples/specification/run/`. Command-facing
projection for generated `EncodeError` result values is also implemented for
`codec.encode_value_unrepresentable`, `codec.dispatch_unknown_tag`,
`codec.dispatch_length_mismatch`, and `codec.dispatch_mismatch`; `veln run`
human output reports focused runtime diagnostics, and `veln run --json`
attaches `details.value_diagnostic` with field path and reason details as
specified under `../specification/run-json.md` and
`../specification/commands.md`. The same command-facing projection is
implemented when a `veln run` entry returns
`EncodeStep::Invalid(EncodeError(...))`, including hand-written codec
`encode with` functions; successful `Encoded` and `Partial` entry values
remain ordinary values. Command-facing projection for
`DecodeStep::Invalid(DecodeError(...))` entry results is implemented;
`veln run` human output reports a focused runtime diagnostic at the contained
byte offset with field-path, optional reason, and source-visible value notes,
and `veln run --json` attaches
`details.byte_diagnostic` with the contained id, byte offset, field path, and
optional reason. For hand-written codec invalid-input failures, a
`DecodeErrorWithReason` reason that is a byte-helper failure message with
registered helper context also projects local byte offset, expected and
available byte counts, and bounded byte preview
as specified under `../specification/run-json.md`,
`../specification/commands.md`, and `../specification/execution.md` and
checked by
`../../examples/specification/run/codec-decode-invalid-byte-context-json/`,
`../../examples/specification/run/codec-decode-invalid-byte-context-human/`,
`../../examples/specification/run/codec-decode-invalid-byte-read-context-json/`,
`../../examples/specification/run/codec-decode-invalid-byte-read-context-human/`,
`../../examples/specification/run/codec-decode-invalid-boundary-json/`,
`../../examples/specification/run/codec-decode-invalid-boundary-human/`,
`../../examples/specification/run/codec-decode-error-direct-json/`,
`../../examples/specification/run/codec-decode-error-direct-human/`,
`../../examples/specification/run/codec-decode-error-owned-id-direct-json/`,
`../../examples/specification/run/codec-decode-error-owned-id-direct-human/`,
`../../examples/specification/run/codec-decode-error-reason-direct-json/`,
`../../examples/specification/run/codec-decode-error-reason-direct-human/`,
`../../examples/specification/run/codec-decode-invalid-owned-id-json/`,
`../../examples/specification/run/codec-decode-invalid-owned-id-human/`,
`../../examples/specification/run/codec-decode-invalid-reason-step-json/`,
`../../examples/specification/run/codec-decode-invalid-reason-step-human/`,
`../../examples/specification/run/codec-decode-invalid-step-json/`, and
`../../examples/specification/run/codec-decode-invalid-step-human/`.
The codec-owned checksum mismatch diagnostic slice is implemented for
`codec.checksum_mismatch` direct `DecodeErrorWithReason(...)` failures and
`DecodeStep::Invalid(DecodeErrorWithReason(...))` failures. It carries
field path, expected checksum, actual checksum, and failure reason in
`details.byte_diagnostic`, keeps the human primary focused on the checksum
mismatch at the byte offset, and is checked by
`../../examples/specification/run/codec-checksum-mismatch-direct-json/`,
`../../examples/specification/run/codec-checksum-mismatch-direct-human/`,
`../../examples/specification/run/codec-checksum-mismatch-step-json/`, and
`../../examples/specification/run/codec-checksum-mismatch-step-human/`.
The completed codec-owned decode invalid id slice is archived under the
[implemented proposal record](../reference/implemented-proposals/codec-owned-decode-invalid-id-diagnostics.md).
Command-facing projection for `DecodeStep::NeedMore(...)` entry results is
implemented as `codec.incomplete_input` at the closed-input reporting
boundary, with readiness and requested byte count details in `run --json` and
focused human diagnostics checked by
`../../examples/specification/run/codec-decode-need-more-json/` and
`../../examples/specification/run/codec-decode-need-more-human/`, plus the
`NeedEnd` readiness cases in
`../../examples/specification/run/codec-decode-need-end-json/` and
`../../examples/specification/run/codec-decode-need-end-human/`.
Successful `Decoded(...)` entry values remain ordinary values. Hand-written
`decode with` codec item calls project decoded results whose consumed count is
outside the supplied `ByteView` to `codec.consumed_count_invalid`; the current
behavior is specified under `../specification/execution.md` and checked by
`../../examples/specification/run/codec-decode-boundary/`. The command-facing
projection for that codec-owned consumed-count failure is specified under
`../specification/run-json.md`, `../specification/commands.md`, and
`../specification/execution.md` and checked by
`../../examples/specification/run/codec-decode-consumed-count-invalid-json/`
and
`../../examples/specification/run/codec-decode-consumed-count-invalid-human/`.
Generated
exact-width binary schema decode helpers report
`schema.integer_out_of_range` when a structurally decoded integer exceeds the
schema-owned external integer range; the current behavior is specified under
`../specification/run-json.md` and `../specification/execution.md` and checked
by
`../../examples/specification/run/binary-schema-integer-out-of-range-json/`
and
`../../examples/specification/run/binary-schema-integer-out-of-range-human/`.
Generated bounded repeated binary schema fields append an `index` segment to
the structured schema field path when a repeated element truncates; the
current behavior is specified under `../specification/run-json.md`,
`../specification/commands.md`, and `../specification/execution.md` and
checked by
`../../examples/specification/run/binary-schema-repeat-truncated-json/` and
`../../examples/specification/run/binary-schema-repeat-truncated-human/`.
The implemented HTTP/2 projection boundary now also includes an ordinary
source-level `Http2DiagnosticContext` and top-level
`http2_protocol_diagnostic` function in the protocol-core executable example.
Representative command-facing HTTP/2 examples route
`http2.protocol.invalid_frame_kind`,
`http2.protocol.invalid_priority_dependency`,
`http2.peer_limit.frame_size_exceeded`, and
`http2.peer_limit.header_list_size_exceeded` through that function while
preserving the same human diagnostics and `details.protocol_diagnostic` JSON
shape, including the PRIORITY self-dependency payload byte preview. Focused
HTTP/2 protocol examples for stream-id domains, post-GOAWAY stream rejection,
fixed payload lengths, invalid DATA padding, content-length body mismatches,
SETTINGS ACK state, preface failures, continuation ordering,
pending-byte close, and remaining invalid frame-kind slices also route through
that projection boundary. The
continuation-ordering and pending-byte close diagnostics carry structured
protocol-owned byte previews for the inspected incoming frame header and
retained pending bytes. The remaining proposal work covers broader schema and
codec diagnostics beyond these implemented slices. The language-level
transport for attaching runtime diagnostic details to failures is covered by
[Runtime Diagnostic Payloads](../reference/implemented-proposals/runtime-diagnostic-payload.md).

## Problem

Existing diagnostics cover source parsing, type checking, effects, contracts,
holes, commands, and runtime failures. Binary schema and HTTP/2 protocol work
needs additional structured context:

- byte offsets
- schema field paths
- expected widths and lengths
- actual available byte counts
- decoded tag values
- related settings or configured limits
- connection and stream state at protocol failure sites

Without this shape, tests can only match broad error strings and agents cannot
repair fixtures or implementations reliably.

## Scope

Define diagnostic support for:

- schema structural failures
- codec incomplete-input reports
- additional codec invalid-input ids beyond the implemented
  command-facing `DecodeError(...)`, `DecodeErrorWithReason(...)`, and
  `DecodeStep::Invalid(...)` slices
- integer conversion overflow at byte boundaries
- schema field paths
- byte offsets and bounded-buffer offsets
- related notes for settings, limits, and source of protocol rules
- protocol-state failures as peer errors or implementation contract failures

## Discussion Result: Protocol Error Boundary

Protocol-state failures should be represented first as ordinary Veln ADTs and
then projected into standard diagnostic data when a caller chooses to report
them.

The diagnostic boundary should require stable fields for the reported fact:
diagnostic id, primary byte offset or source span when one exists, focused
human message, and related notes for stream id, frame kind, current state,
active setting, configured limit, and rule provenance. The ADT carries the
domain-specific protocol meaning; the projection carries the stable shape used
by commands, fixtures, JSON output, and agents.

This proposal should not require a global protocol-error supertype. A protocol
module can define its own error ADT and an explicit conversion into diagnostic
data for the reporting surfaces it supports.

## Discussion Result: Related Diagnostic Context

The primary human message should state only the failed fact at the reported
source span or byte position. Related notes are required when a useful fact
does not itself fail at the primary location, including rule provenance,
decoded field values, active settings, configured limits, current connection or
stream state, and repair hints.

Schema and codec diagnostics should use related notes for context such as the
field path, expected width or length, actual available byte count, decoded tag
value, and surrounding byte preview when those details would otherwise make the
primary message describe more than one fact. Protocol-state diagnostics should
use related notes for stream id, frame kind, previous state, active setting,
configured limit, and whether the failure is blamed on a peer protocol error or
an implementation contract.

Human output may render related notes near the primary message, and JSON output
should keep them as structured entries so agents and fixtures can assert the
specific context they need. A related note is optional only when the same fact
is already represented by a stable primary field and repeating it would add no
repair value.

## Discussion Result: Schema Field Path Shape

Schema diagnostics should represent a field path as an ordered list of
schema-local path segments plus an optional human display string. The stable
JSON field is `field_path`, whose value is a list of segment objects. Each
segment records its kind and name or index so fixtures and agents do not need
to parse a localized display string.

The first segment is the schema declaration name. Ordinary field segments use
the written schema-local field name, including `_`-prefixed representation
fields such as reserved bits. Dispatch payloads add a segment for the selected
tag or case before entering the nested schema fields. Implemented bounded
repeated binary schema fields append numeric index segments at the point where
the repeated element is entered.

Human output may render the same path in a compact dotted form such as
`Http2FrameHeader.stream_id`, but that spelling is presentation only. JSON
output keeps both the structured `field_path` and, when useful for logs, a
`field_path_display` string. Source module qualification is reported separately
when relevant; it is not folded into schema-local field paths.

Field paths name representation locations, not mapped Veln value fields. A
schema mapping may rename or omit representation fields, but diagnostics for
decode, validation, reserved bits, dispatch, and incomplete input continue to
point at the schema-local representation path that owned the failed byte or
predicate.

## Discussion Result: Byte Offset Reporting

Schema, codec, and protocol byte diagnostics should report decoded-stream
locations as zero-based absolute `ByteOffset` values. The offset is measured
from the start of the byte stream owned by the parser state, not from the
currently retained buffer and not from fixture source text.

Human output should include the byte location with the primary diagnostic
location, using wording such as `byte offset N`. If the diagnostic also has a
source span, such as fixture text that produced the bytes, that source span is
related context unless the failed fact is about the source text itself. Codec
and protocol failures after successful fixture decoding should keep their
primary location on the decoded byte stream so fixture formatting changes do
not rewrite expected locations.

JSON output should not overload source `span.offset` for decoded byte
positions. Byte diagnostics use a stable `details.byte_offset` field for the
absolute `ByteOffset`; `span` remains the source span or `null` according to
the ordinary diagnostic envelope. When a diagnostic also needs local parser
state, `details` may carry `base_offset`, `local_offset`, `expected_count`,
`actual_count`, or `available_count` as separate numeric fields, but fixtures
and agents should assert the absolute `byte_offset` for the reported failure.

For incomplete input, the reported byte offset is the first missing byte when
the failure can identify one; otherwise it is the absolute offset of the field
or frame boundary that required more input. Byte previews, consumed counts,
and retained-buffer sizes remain related data or separate detail fields rather
than being folded into the byte offset text.

## Discussion Result: Schema And Codec Diagnostic Ids

Schema-owned structural failures should use narrow `schema.*` diagnostic ids
that name the failed representation fact. The first canonical ids are:

- `schema.fixed_field_mismatch` for fixed fields whose decoded value differs
  from the schema requirement
- `schema.reserved_bits_mismatch` for reserved bit fields whose actual bit
  pattern differs from the required pattern; the frame-header
  `ReservedBits(1, 0)` slice is implemented under `../specification/run-json.md`
- `schema.truncated_field` for a schema field whose representation bytes are
  unavailable; the generated frame-header helper slice is implemented under
  `../specification/run-json.md`
- `schema.validation_failed` for a field-local schema `where` predicate that
  evaluates to false; the first narrow validation slice is implemented under
  `../specification/run-json.md`
- `schema.dispatch_unknown_tag` for an unknown tag in a closed dispatch
- `schema.length_out_of_bounds` for a decoded length or count that cannot be
  sliced from the available bounded input
- `schema.integer_out_of_range` for a value that cannot be represented by the
  schema-owned external integer width; the generated exact-width binary schema
  decode slice is implemented under `../specification/run-json.md`

Codec-owned failures should use `codec.*` ids only when the failed fact belongs
to executable decode or encode behavior rather than to the schema
representation itself. The first canonical ids are:

- `codec.incomplete_input` for a pending `NeedMore` readiness that becomes
  reportable because the caller supplied end-of-stream or used a closed-input
  helper
- `codec.consumed_count_invalid` for a decoder result whose consumed
  `ByteCount` is outside the supplied `ByteView`
- `codec.encode_value_unrepresentable` for a value that cannot be emitted by
  an encoder because the selected codec direction, mapping, or representation
  checks cannot produce bytes

Do not collapse schema failures into a generic `codec.invalid_input` when the
schema can name the failed field fact. A hand-written codec may project an
invalid byte sequence through a codec-specific id only when no schema-owned id
applies. All of these ids keep the primary message focused on the failed fact
and carry byte offset, field path, expected values, actual values, readiness,
and byte counts through structured details or related notes.

## Discussion Result: Incomplete Input Command Output

Command output should distinguish incomplete input from invalid input by
readiness and blame, not by broad wording such as "bad input".

While a command is exercising an incremental stream, a `NeedMore` transition is
ordinary progress and should not emit a diagnostic or set a failure exit status.
The command may expose the pending readiness in machine-readable trace or
fixture output, using a shape that records the readiness kind, minimum required
`ByteCount` when known, current available count, absolute `ByteOffset`, and
field path when available.

When a closed-input helper or end-of-stream event makes pending readiness
reportable, the command emits `codec.incomplete_input`. The human primary
message names the missing byte fact, such as an incomplete field or payload at
the first missing byte offset. Related notes carry the readiness that was
pending, available byte count, expected count, field path, and whether the
input was closed by a fixture helper or by stream end. JSON output should use
`details.readiness = "need_bytes"` or `"need_end"` plus separate numeric count
fields rather than encoding those facts into a display string.

Invalid input means the supplied bytes were sufficient to prove a schema,
codec, or protocol fact false. Commands report the narrow `schema.*`,
`codec.*`, or protocol diagnostic id for that failed fact, keep the byte offset
on the offending byte or field, and do not present the failure as retryable
readiness. Invalid-input JSON may include the same available and expected
counts when useful, but it must not set a readiness field that suggests more
bytes could make the same bytes valid.

This split lets fixture runners assert truncated input separately from malformed
input: truncation is a closed-input projection of pending readiness, while
invalid input is a failure already established by the bytes that were present.

## Discussion Result: Explicit Protocol Diagnostic Projection

Protocol modules should expose diagnostic projection as an ordinary pure
top-level function, not as a trait, automatic derive, implicit conversion, or
diagnostic-emitting return type.

The source shape should take the protocol module's own error ADT plus a
protocol-specific reporting context, then return standard diagnostic data:

```text
fn http2_protocol_diagnostic(
	error: Http2ProtocolError,
	context: Http2DiagnosticContext,
) -> Diagnostic
```

The error ADT remains the value used by pure protocol transitions for recovery
decisions, such as whether to close a connection, reset a stream, continue
processing, or emit response frames. The diagnostic context carries reporting
facts that are not always part of the recovery value: absolute byte offset or
source span, decoded frame kind, stream reference, current state summary,
active setting, configured limit, fixture name, and rule provenance.

The projection function owns the stable diagnostic id, severity, primary
message, structured details, and related notes for each protocol error variant.
It must keep the primary message focused on the failed protocol fact and put
state, settings, limits, and provenance into related notes or structured
details. It may call shared diagnostic helper functions, but it should not read
transport state, inspect sockets, mutate protocol state, or decide recovery.

Commands, fixture helpers, and adapters call the projection function only when
they choose to report a protocol error. Returning `Http2ProtocolError` from a
pure transition does not emit a diagnostic by itself. This keeps protocol logic
typed and recoverable while giving reporting surfaces a single explicit route
to stable human and JSON diagnostics.

## Non-Goals

- Do not define full binary schema syntax.
- Do not implement HTTP/2 state machines.
- Do not replace existing source, type, effect, or contract diagnostic shapes.

## Completion Criteria

The implemented diagnostic slices cover closed-input `ByteView` read
truncation as `codec.incomplete_input`, fixed-field mismatches, frame-header
schema truncation, reserved-bit mismatches, and payload length boundary
failures, command-facing direct `DecodeError(...)`,
`DecodeErrorWithReason(...)`, `DecodeStep::Invalid(DecodeError(...))`,
`DecodeStep::Invalid(DecodeErrorWithReason(...))`, hand-written
codec-owned invalid-input ids, and `DecodeStep::NeedMore(...)` entry
projections, plus HTTP/2 protocol-state
projections for frame-size peer-limit,
SETTINGS value range peer-limit, DATA receive flow-control peer-limit, client
connection preface failures, and invalid connection-state and stream-state
frame-kind failures, stream id domain failures, post-GOAWAY stream failures,
plus fixed payload-length failures for SETTINGS ACK, PING, GOAWAY,
`RST_STREAM`, and `WINDOW_UPDATE`, plus invalid DATA padding and
content-length body mismatches. DATA receive flow-control, fixed
payload-length failures, invalid DATA padding, and content-length body
mismatches include protocol-owned payload byte previews.
`run --json` examples assert the stable byte and protocol diagnostic detail
fields documented in `../specification/run-json.md`; human `run` examples
assert the focused primary messages and related notes documented in
`../specification/commands.md`.

The implemented protocol-state diagnostic projection slice covers:

- Protocol-state examples that cover invalid stream id domains and invalid
  frame kind for a connection or stream state, including the idle-stream rule
  that expects HEADERS before DATA. Stream id domain projection includes a
  bounded protocol-owned frame-header byte preview.
- Post-GOAWAY stream-state examples that reject new peer-created streams above
  the recorded last stream id.
- Schema and protocol diagnostics that keep the primary message focused on the
  failed fact at the reported span or byte position.
- Related notes that carry provenance, settings, limits, and state-transition
  context where those surfaces are implemented.
- HTTP/2 design-driver support for valid and invalid binary fixtures with
  stable diagnostic assertions.

The proposal remains open for broader schema and codec diagnostics that are not
specified as current behavior under `../specification/`, including additional
codec diagnostic ids outside the implemented `EncodeError`,
`codec.invalid_input`, `codec.packet_kind_invalid`, and
`codec.consumed_count_invalid` command-facing slices.
