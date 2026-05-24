# Discussion Result: JSON Diagnostic Schema Stability

Status: implemented

## Picked Question

- Should JSON diagnostics use a stable schema before the language is stable, or
  should the prototype allow schema churn?

## Decision

Use a stable top-level diagnostic envelope from the first slice, while allowing
kind-specific `details` payloads to change during the prototype.

The top-level JSON shape is part of the agent-facing product surface. Agents,
editors, CI scripts, and tests should be able to route diagnostics, detect
incomplete analysis, and decide what to run next without rewriting parsers for
every language experiment. The language is still expected to change, so the
unstable parts should be isolated below each diagnostic kind.

## First-Slice Stable Fields

- `schema_version`: integer version for the top-level envelope.
- `tool`: tool name and version.
- `status`: `ok`, `error`, or `partial`.
- `diagnostics`: ordered array of diagnostic objects.
- `summary`: counts by severity and kind.

## First-Slice Stable Diagnostic Fields

- `id`: stable diagnostic code, such as `hole.expected_type`.
- `severity`: `error`, `warning`, `info`, or `hint`.
- `kind`: broad category, such as `parse`, `module`, `type`, `contract`,
  `effect`, `lint`, `hole`, or `doc`.
- `message`: short human-readable message.
- `span`: file, start, and end positions when source-backed.
- `details`: kind-specific payload, explicitly allowed to change until the
  corresponding language feature is promoted from prototype status.
- `related`: optional related spans, tests, contracts, or candidate fixes.

## Compatibility Rule

Breaking changes to the stable envelope require a `schema_version` bump. Changes
inside `details` do not require a top-level bump while the diagnostic kind is
marked prototype, but each detail payload should include enough structure for
tests to assert important behavior without matching prose.

## Consequence

`veln check --json` can become the first reliable integration point without
freezing the whole language design too early.
