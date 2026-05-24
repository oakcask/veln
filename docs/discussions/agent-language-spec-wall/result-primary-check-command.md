# Discussion Result: Primary Check Command

## Picked Question

- Should `check` be the primary agent command that combines parse, type,
  contract, effect, lint, doc drift, and hole diagnostics?

## Decision

Make `veln check` the primary read-only command for agents. It should run every
available static analysis pass that can produce useful diagnostics without
executing user code, and it should return one ordered diagnostic stream through
the stable JSON envelope.

## Rationale

Agents need one low-risk command that answers "what is wrong, what can I trust,
and what should I inspect next?" If parse, type, contract, effect, lint, doc
drift, and hole queries live behind separate first-line commands, the agent has
to learn orchestration policy before it can repair a program. A combined
`check` command keeps the repair loop smaller and makes editor, CI, and test
integrations converge on one contract.

The command should still expose analysis boundaries. A parser failure should not
pretend that type, contract, or effect analysis completed. Later passes should
run when the earlier representation is good enough, and skipped or degraded
passes should be visible in the JSON `summary` and `details` for the relevant
diagnostics.

## First-Slice Rule

- `veln check` is read-only and must not execute user code.
- `veln check --json` returns the stable diagnostic envelope even when only
  parse diagnostics are available.
- The command reports `status: partial` when one or more requested analysis
  passes could not run.
- Diagnostics from later passes may be omitted after blocking parse or binding
  errors, but the summary should say which passes were skipped.
- Specialized commands may exist later, but they should be explainability or
  workflow shortcuts over the same underlying analysis results.

## Consequence

The first implementation can give agents a single default repair-loop command
without freezing every analysis pass or requiring an early full dependency
graph.
