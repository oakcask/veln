# Runtime Diagnostic Payloads

Status: partially implemented

This proposal gives runtime diagnostics an explicit language-level position.
Runtime failures that carry stable diagnostic facts should be represented as
ordinary `Err` values, then projected by command surfaces into human
diagnostics and JSON details. Runtime logs, metrics, and backend-local side
tables are not the semantic transport for those facts. The purpose is to
retire the current backend side-table bridge and make diagnostic-bearing
failures part of ordinary value flow.

## Problem

Current executable diagnostics can attach structured result details such as
fixture hex spans, byte diagnostics, value diagnostics, and protocol
diagnostics. The observable command shape is useful, but the implementation
can rely on backend-local state that is keyed by the rendered `Err` value.

That makes the diagnostic payload separate from the failing value:

- a helper can return `Err(message)` while registering details elsewhere
- the command recorder later reconnects details by matching the rendered
  message
- repeated equal messages can collide inside the same execution context
- diagnostics registered on one execution thread are not naturally visible to
  another recorder thread
- source modules cannot construct or pass diagnostic-bearing failures without
  calling narrow backend helpers

This is acceptable as a compatibility bridge, but it should not be the
language model. The design debt is that command-facing diagnostic details are
not carried by the `Err` value that caused the command failure.

## Goals

- Treat runtime diagnostic details as typed data that belongs to the reported
  failure, not as logs, metrics, or backend-local recorder state.
- Let protocol and fixture modules define ordinary domain error ADTs first,
  then explicitly project those errors into stable `Err(RuntimeDiagnostic(...))`
  values at a reporting boundary.
- Keep `Result` as the transport for command-facing failures instead of adding
  a parallel diagnostic result wrapper.
- Keep command output stable: `veln run`, `veln run --json`, and test harness
  reports continue to expose focused runtime diagnostics and structured
  `details.*` objects.
- Allow HPACK fixture and HTTP/2 protocol helpers to move domain-specific
  diagnostic construction out of backend-specific runtime functions.
- Preserve ordinary `Result<T, E>` behavior for failures that do not opt into a
  diagnostic payload.

## Non-Goals

- Do not make logs or metrics part of the language semantics. They may observe
  diagnostics, but they do not define command results.
- Do not require one global protocol-error supertype. Modules can keep their
  own domain ADTs.
- Do not require every `Err` value to have a diagnostic payload.
- Do not introduce a second result-like type only for diagnostics.
- Do not expose backend trace-file formats as source-visible API.

## Proposed Model

Add a source-visible diagnostic payload vocabulary that can be returned as the
error value of ordinary `Result` failures at explicit projection boundaries.

A diagnostic payload has:

- `id`: the stable diagnostic identifier
- `message`: the focused primary message
- `kind`: the payload family, such as byte, value, fixture, or protocol
- an optional byte offset or source span
- structured fields used by JSON output and related human notes
- optional bounded byte preview data with stable encoding fields

The exact surface can be introduced as a narrow standard-library vocabulary
rather than special syntax. One possible shape is:

```veln
pub type RuntimeDiagnostic
	RuntimeDiagnostic(id: String, message: String, detail: RuntimeDiagnosticDetail)
end
```

The names above are illustrative. The important rule is that the diagnostic
payload is the `Err` value, or is contained directly in the `Err` value.
Command execution should not need to rediscover details from a backend-local
side table keyed by rendered text.

Plain error values continue to work:

```veln
pub fn ordinary_failure() -> Result<(), String>
	Err("bad input")
end
```

Diagnostic-bearing failures use the same `Result` construct with a structured
error type:

```veln
pub fn reported_failure() -> Result<(), RuntimeDiagnostic>
	Err(RuntimeDiagnostic("hpack.fixture.malformed_raw_string_value", "...", detail))
end
```

## Projection Boundary

Protocol and fixture modules should keep domain failures separate from command
diagnostics:

```veln
pub type HpackFixtureFailure
	MalformedRawStringValue(offset: Int, observed_size: Int, preview: ByteChunk)
	UnsupportedHeaderBlock(offset: Int, observed_size: Int, preview: ByteChunk)
end

pub fn project_failure(failure: HpackFixtureFailure) -> Result<(), RuntimeDiagnostic>
	# Converts the domain error into a stable diagnostic Err value.
end
```

Returning `HpackFixtureFailure` by itself remains an ordinary value. Calling
the projection boundary converts it to a command-facing `Err` value with
stable diagnostic fields.

This preserves the current distinction used by protocol-core examples:
ordinary protocol errors are values until a command, fixture helper, or adapter
reports them through a projection helper.

## Command Semantics

When an entry returns `Err(value)`, command surfaces inspect the error value.
If the value has the standard runtime diagnostic shape, command surfaces render
it as a runtime result failure with structured details:

- human output uses the diagnostic id, message, and related notes
- JSON output uses the existing `error.kind: "result"` shape and attaches the
  matching `details.fixture_hex`, `details.byte_diagnostic`,
  `details.value_diagnostic`, or `details.protocol_diagnostic` object
- tests match the public JSON and human output, not a backend-local trace
  encoding

When an entry returns a plain `Err(value)`, command surfaces keep the current
plain result-failure behavior.

This keeps the difference at the value level. No command needs to observe an
out-of-band registration event to know whether an `Err` carries diagnostic
details.

## Implemented Baseline

Current behavior for the completed source-visible byte diagnostic value,
command projection, and executable harness assertion slices is specified in
`../specification/run-json.md`, `../specification/commands.md`,
`../specification/execution.md`, and `../specification/test-json.md`.

The remaining proposal work starts from that baseline. New migration slices
should add executable examples under `../../examples/specification/run/`
before updating specification prose.

## Logs And Metrics

Logs and metrics may be derived from runtime diagnostics, but they are not the
transport.

Runtime diagnostics describe the failed program result. They need stable ids,
byte offsets, field paths, provenance, bounded previews, and related facts.
Those facts are part of the command result and must survive formatting,
testing, and backend changes. Logs are secondary streams, and metrics are
aggregate observations; neither is a reliable carrier for per-failure semantic
data.

## Backend Implications

Backend-local diagnostic side tables are the design debt this proposal intends
to remove. They may remain temporarily for legacy helpers, but new helpers
should return source-visible diagnostic error values.

The first implemented slice defines the standard `RuntimeDiagnostic` value
shape for byte diagnostics and lets `veln run` and `veln run --json` project
`Err(RuntimeDiagnostic(id, message, RuntimeByteDiagnostic(...)))` through the
same human and JSON byte-diagnostic surfaces used by legacy helpers. The
checked examples are
`../../examples/specification/run/runtime-diagnostic-payload-byte-human/`,
`../../examples/specification/run/runtime-diagnostic-payload-byte-json/`, and
`../../examples/specification/run/runtime-diagnostic-payload-plain-json/`.
This slice deliberately leaves legacy side-table support in place for existing
fixture, value, protocol, HPACK, HTTP/2, and generated-schema helpers.

A staged migration can keep compatibility for the remaining work:

1. Convert HPACK fixture projection helpers to return `Result<(), RuntimeDiagnostic>`
   or an equivalent structured diagnostic error type from Veln.
2. Convert HTTP/2 protocol projection helpers to the same model.
3. Remove narrow backend helpers and side-table registrations once no
   specification case depends on them.

During migration, existing Java runtime helpers can keep producing the same
public output. The specification should describe the payload semantics, not
the backend-local storage mechanism.

The migration is complete when diagnostic details are reachable from the
failing `Err` value itself and command recording no longer needs a
message-keyed store to attach public result details.

## Remaining Specification Updates

For each remaining migration slice, update the smallest matching current
specification route after executable evidence exists:

- `../specification/run-json.md` for new result-failure JSON projections
- `../specification/commands.md` for new human command diagnostics
- `../specification/execution.md` for new runtime failure projection semantics
- executable examples under `../../examples/specification/run/`

Completed rationale should then be archived under
`../reference/implemented-proposals/`.

## Discussion Results

Diagnostic-bearing failures should use ordinary `Result` values with a standard
error ADT convention. The proposal should not introduce a dedicated diagnostic
result wrapper. A function that wants command-facing diagnostics returns
`Err(RuntimeDiagnostic(...))` or an equivalent standard diagnostic error value;
functions that do not opt in keep returning plain `Err(value)`.

Payload details should be represented by a small closed set of detail
constructors rather than a record-like map. The closed constructor set names
the stable diagnostic family, such as byte, value, fixture, or protocol, while
each constructor carries the structured fields needed by JSON output, related
human notes, fixtures, and agents. This keeps the value typed without freezing
one global field list for every diagnostic family.

Diagnostic payloads should not compose by automatically merging nested
payloads. A source module may wrap one domain error inside another as ordinary
data, but the reporting boundary selects one primary diagnostic payload. Inner
causes, provenance, and wrapped domain values belong in structured detail
fields or related notes when they help explain the reported fact.

Any function may return a diagnostic-bearing failure as an ordinary value.
Projection into command-facing diagnostics still happens only when a command,
fixture helper, adapter, or other reporting boundary observes the standard
diagnostic error value. Returning a domain error ADT by itself remains
non-diagnostic until an explicit projection function converts it.
