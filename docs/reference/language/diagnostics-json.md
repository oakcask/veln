# Check JSON

This is the routing page for the implemented `veln check --json` output.

## Read First

- Top-level envelope and status values:
  [diagnostics-json-full.md](diagnostics-json-full.md#envelope).
- Common diagnostic fields and span shape:
  [diagnostics-json-full.md](diagnostics-json-full.md#diagnostics).
- Stable `details` payloads by diagnostic family:
  [diagnostics-json-full.md](diagnostics-json-full.md#stable-details).

## Read When

- Adding, removing, or changing `check --json` fields.
- Updating human diagnostics that also need structured output coverage.
- Verifying whether diagnostic provenance, repair hints, or related notes are
  stable machine-readable behavior.

## Skip Unless Needed

- Do not read the full details catalog before the envelope and diagnostic
  family are relevant to the task.
- Use [commands.md](commands.md) for CLI behavior and
  [test-json.md](test-json.md) or [run-json.md](run-json.md) for other command
  JSON surfaces.

## Envelope

See [diagnostics-json-full.md#envelope](diagnostics-json-full.md#envelope).

## Diagnostics

See
[diagnostics-json-full.md#diagnostics](diagnostics-json-full.md#diagnostics).

## Stable Details

See
[diagnostics-json-full.md#stable-details](diagnostics-json-full.md#stable-details).
