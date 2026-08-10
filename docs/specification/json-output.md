---
role: specification
authority: normative
update-when: A command JSON schema, output field, or executable JSON evidence changes.
---

# JSON Output

This file routes machine-readable command output changes to the smallest
implemented specification page. Use it before opening command-specific JSON
details.

## Read First

- `check --json`, `metrics --json`, `run --json`, `test --json`, and
  `repair --json` keep their command-specific envelopes while reusing the
  relevant command analysis path described by [commands.md](commands.md).
- Help output is human command output, not command JSON. Use
  [commands.md](commands.md) for top-level and subcommand help behavior.
- Human diagnostic wording, related notes, spans, or stable diagnostic details:
  [diagnostics-json.md](diagnostics-json.md). Keep human output and structured
  output aligned before checking command-specific behavior.
- `check --json`: [diagnostics-json.md](diagnostics-json.md) for diagnostic
  envelope, span, related note, and stable details fields.
- `metrics --json`, `metrics --check --baseline PATH --json`, and
  `metrics --write-baseline PATH`: [metrics-json.md](metrics-json.md) for
  module dependency metrics, dependency edges, cycles, baseline documents, ABC
  size subjects, experimental exact whole-body similarity records, project
  identity, and summary fields.
- `run --json`: [run-json.md](run-json.md) for run records, output events,
  failures, and summary shape.
  Runtime result failures that carry source-visible diagnostic payload values,
  including `RuntimeDiagnostic(..., RuntimeByteDiagnostic(...))`, are specified
  there.
  HTTP/2 application-boundary cases that distinguish callback, unsupported
  request, invalid action, and rejected core-action outcomes reuse the same
  run envelope; their source-visible observations are routed from
  [http2.md](http2.md).
- `test --json`: [test-json.md](test-json.md) for selection, case, summary,
  failure, and error records.
- `repair --json`: [repair-json.md](repair-json.md) for preview, apply,
  refusal, candidate, edit, verification, and summary records.

## Read When

- Adding, removing, or renaming machine-readable output fields.
- Changing diagnostic details, provenance, related notes, or repair data that
  must stay stable for tools.
- Updating command behavior where human output and JSON output must stay
  aligned.

## Skip Unless Needed

- Use [commands.md](commands.md) first when the task is about CLI gates,
  source discovery, entry selection, or exit behavior.
- Use [diagnostics-json.md](diagnostics-json.md) before a broader command page
  when only diagnostic fields or related notes change.
- Use the `*-full.md` files only after the short JSON page points to the
  relevant section.
