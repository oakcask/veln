---
review-when: The documented diagnostics JSON behavior or executable diagnostic evidence changes.
---

# Check JSON And Diagnostics

This is the routing page for implemented `veln check --json` output and
human diagnostics that must stay aligned with structured diagnostic behavior.

## Read First

- Human primary messages: keep the primary message focused on the failed fact
  at the reported span; put causes, provenance, repair hints, and other
  locations in related notes.
- Top-level envelope and status values:
  [diagnostics-json-full.md](diagnostics-json-full.md).
- Common diagnostic fields and span shape:
  [diagnostics-json-full.md](diagnostics-json-full.md).
- Stable `details` payloads by diagnostic family:
  [diagnostics-json-full.md](diagnostics-json-full.md).
- Companion source diagnostics distinguish missing targets from chained
  companions and expose `details.companion_path` plus
  `details.target_path`.
- Local inference diagnostic details:
  [diagnostics-json-full.md#type-inference-diagnostics](diagnostics-json-full.md#type-inference-diagnostics).
- Advisory repair candidate fields and application-policy routing:
  [repair-candidates.md](repair-candidates.md).

## Read When

- Adding, removing, or changing `check --json` fields.
- Updating human diagnostics that also need structured output coverage.
- Verifying whether diagnostic provenance, repair hints, or related notes are
  stable machine-readable behavior.
- Changing hole candidate `details` payloads, candidate edits, or
  application-policy fields.

## Skip Unless Needed

- Do not read the full details catalog before the envelope and diagnostic
  family are relevant to the task.
- Use [json-output.md](json-output.md) when choosing between `check --json`,
  `run --json`, and `test --json`.
- Use [commands.md](commands.md) for CLI behavior and
  [test-json.md](test-json.md) or [run-json.md](run-json.md) for other command
  JSON surfaces.

## Envelope

See [diagnostics-json-full.md](diagnostics-json-full.md).

## Diagnostics

See
[diagnostics-json-full.md](diagnostics-json-full.md).

Malformed binary and hexadecimal integer coverage is executable in
`examples/specification/check/integer-radix-diagnostics-json/` and the matching
human-output case. The `parse.integer_literal` details retain the complete
numeric candidate, parser context, accepted form, and non-cascading recovery;
related notes expose the accepted digit set or prefix where useful.

Invalid literal shift counts use `type.invalid_shift_count` with the operator,
actual count, and inclusive `0..63` bounds. Removed schema primitives, types,
constructors, patterns, and helpers use focused removed-vocabulary diagnostics
with replacement details instead of generic unresolved-name output.

Companion source diagnostics are executable in
`examples/specification/check/companion-missing-target-json/`,
`examples/specification/check/companion-missing-target-human/`,
`examples/specification/check/companion-chained-target-json/`,
`examples/specification/check/companion-chained-target-human/`,
`examples/specification/test/companion-missing-target-json/`,
`examples/specification/test/companion-missing-target-human/`,
`examples/specification/test/companion-chained-target-json/`, and
`examples/specification/test/companion-chained-target-human/`.

## Stable Details

See
[diagnostics-json-full.md](diagnostics-json-full.md).
