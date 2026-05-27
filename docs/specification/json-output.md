# JSON Output

This file routes machine-readable command output changes to the smallest
implemented specification page. Use it before opening command-specific JSON
details.

## Read First

- Human diagnostic wording, related notes, spans, or stable diagnostic details:
  [diagnostics-json.md](diagnostics-json.md). Keep human output and structured
  output aligned before checking command-specific behavior.
- `check --json`: [diagnostics-json.md](diagnostics-json.md) for diagnostic
  envelope, span, related note, and stable details fields.
- `run --json`: [run-json.md](run-json.md) for run records, output events,
  failures, and summary shape.
- `test --json`: [test-json.md](test-json.md) for selection, case, summary,
  failure, and error records.

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
