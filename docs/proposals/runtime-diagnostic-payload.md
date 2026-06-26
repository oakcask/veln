# Runtime Diagnostic Payloads

Status: proposed

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
- Let executable specification cases inspect the returned `Err` value shape
  directly, so they can prove diagnostics are carried by values rather than by
  backend side effects.
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

## Test Harness Semantics

The executable specification harness should be able to inspect the structure
of an entry result failure before or alongside command projection. Public
output assertions still check human diagnostics and `run --json`, but
lower-level harness assertions should also be able to prove that the entry
returned a diagnostic-bearing `Err` value.

The harness should support path assertions over the error value, including:

- the outer result constructor, such as `Err`
- the diagnostic payload constructor, such as `RuntimeDiagnostic`
- stable fields such as `id`, `message`, payload family, byte offset, field
  path, provenance, and bounded byte preview fields
- ordinary non-diagnostic error values, which remain inspectable as plain
  returned values

For example, a case should be able to assert that an entry returned
`Err(RuntimeDiagnostic(...))` with id
`hpack.fixture.malformed_raw_string_value`, independently of the projected
human or JSON command output.

This keeps the test evidence aligned with the language model. Tests should
not need to observe a backend trace side table, a thread-local diagnostic
registration, or a rendered `Err` string to prove that a diagnostic payload was
attached to the failure.

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

A staged migration can keep compatibility:

1. Define the standard-library payload vocabulary and command projection.
2. Add command support for `Err(RuntimeDiagnostic(...))` and any accepted
   diagnostic error ADT conventions.
3. Add harness assertions over structured `Err` values so implementation tests
   can verify value-carried diagnostics before command rendering.
4. Convert HPACK fixture projection helpers to return `Result<(), RuntimeDiagnostic>`
   or an equivalent structured diagnostic error type from Veln.
5. Convert HTTP/2 protocol projection helpers to the same model.
6. Remove narrow backend helpers and side-table registrations once no
   specification case depends on them.

During migration, existing Java runtime helpers can keep producing the same
public output. The specification should describe the payload semantics, not
the backend-local storage mechanism.

The migration is complete when diagnostic details are reachable from the
failing `Err` value itself and command recording no longer needs a
message-keyed store to attach public result details.

## Specification Updates

When implemented, current behavior should move into:

- `../specification/run-json.md` for result-failure JSON shape
- `../specification/commands.md` for human command diagnostics
- `../specification/execution.md` for runtime failure projection semantics
- `../specification/test-json.md` for harness-visible `Err` value assertions
- executable examples under `../../examples/specification/run/`

Completed rationale should then be archived under
`../reference/implemented-proposals/`.

## Open Questions

- Should diagnostic-bearing failures use a dedicated result wrapper or a
  standard error ADT convention? This proposal prefers the standard error ADT
  convention unless implementation work exposes a concrete blocker.
- Should payload details be a fixed ADT family, a record-like map, or a small
  closed set of detail constructors?
- How should diagnostic payloads compose when a source module wraps one domain
  error inside another?
- Should projection be allowed only at command-facing boundaries, or should any
  function be able to return diagnostic-bearing failures as ordinary values?
