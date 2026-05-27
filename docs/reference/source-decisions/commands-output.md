# Commands And Output Decisions

Read these records only when command, JSON, test selection, or observable I/O
behavior needs rationale or compatibility context.

## Read First

- Current CLI behavior: [../../specification/commands.md](../../specification/commands.md).
- Current machine-readable output route:
  [../../specification/json-output.md](../../specification/json-output.md).
- Current runtime output behavior:
  [../../specification/execution.md](../../specification/execution.md).

## Read When

- Use the sections below only after the implemented command or JSON page names
  a boundary but does not explain why it exists.
- Open an individual `result-*.md` record only for the selected command,
  output, test, or runtime-output topic.

## Commands And Discovery

- [First Implementation Commands](records/result-first-implementation-commands.md)
- [Minimal Project and Test Discovery](records/result-minimal-project-test-discovery.md)
- [Primary Check Command](records/result-primary-check-command.md)

## JSON Output

- [Check JSON Details Fields](records/result-check-json-details-fields.md)
- [Hole Diagnostic JSON Shape](records/result-hole-diagnostic-json-shape.md)
- [JSON Diagnostic Schema Stability](records/result-json-diagnostic-schema-stability.md)
- [Test JSON Shape](records/result-test-json-shape.md)

## Tests And Doctests

- [Affected Test Selection](records/result-affected-test-selection.md)
- [Doctest Error Type Fence Syntax](records/result-doctest-error-type-fence-syntax.md)
- [Doctest Expected Output Syntax](records/result-doctest-expected-output-syntax.md)
- [Doctest Result Propagation](records/result-doctest-result-propagation.md)

## Runtime Output

- [First-Slice Observable I/O](records/result-first-slice-observable-io.md)
- [Runtime Contract Failure Reporting](records/result-runtime-contract-failure-reporting.md)
- [Stdio API and Output Events](records/result-stdio-api-and-output-events.md)

## Skip Unless Needed

Use [../../specification/commands.md](../../specification/commands.md),
[../../specification/diagnostics-json.md](../../specification/diagnostics-json.md),
[../../specification/run-json.md](../../specification/run-json.md), or
[../../specification/test-json.md](../../specification/test-json.md) before opening these
decision records for implemented behavior.
