# Check JSON And Diagnostics

This is the routing page for implemented `veln check --json` output and
human diagnostics that must stay aligned with structured diagnostic behavior.

## Read First

- Human primary messages: keep the primary message focused on the failed fact
  at the reported span; put causes, provenance, repair hints, and other
  locations in related notes.
- Top-level envelope and status values:
  [diagnostics-json-full.md](diagnostics-json-full.md#envelope).
- Common diagnostic fields and span shape:
  [diagnostics-json-full.md](diagnostics-json-full.md#diagnostics).
- Stable `details` payloads by diagnostic family:
  [diagnostics-json-full.md](diagnostics-json-full.md#stable-details).
- Hole candidate JSON keeps statically satisfied safe repair candidates even
  when ordinary manual-review candidates are bounded.

## Read When

- Adding, removing, or changing `check --json` fields.
- Updating human diagnostics that also need structured output coverage.
- Verifying whether diagnostic provenance, repair hints, or related notes are
  stable machine-readable behavior.

## Skip Unless Needed

- Do not read the full details catalog before the envelope and diagnostic
  family are relevant to the task.
- Use [json-output.md](json-output.md) when choosing between `check --json`,
  `run --json`, and `test --json`.
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
