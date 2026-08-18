---
role: implementation-record
authority: supporting
update-when: The agent-language-services closed client-platform matrix, matrix-reference registry, validator evidence, workflow registration, or lifecycle migration prerequisite changes.
---

# Agent Language Services Platform Matrix Closure

## Summary

The agent-language-services plan now has one finite client-platform matrix for
plugin compatibility planning. The matrix is a documentation contract only. It
does not claim that Codex or Claude Code host validation has passed.

## Completed Contract

The umbrella proposal contains one `Closed Client-Platform Matrix` section under
`Agent Plugin`. That section declares exactly two ordered cells:
`codex/x86_64-unknown-linux-gnu` and
`claude-code/x86_64-unknown-linux-gnu`. Each cell carries exact literal values
for client, platform, host build, manifest schema, validator version,
validator integrity, Veln, MCP, LSP, language-service, and reference-schema
contracts.

The same section declares the thirteen matrix-reference tuples that bind
plugin requirements, Q21 evidence, Q22 totality, proposal completion, and
lifecycle migration prerequisites to the same closed matrix.

The documentation validator rejects hidden, displaced, duplicate, reordered,
malformed, missing-field, empty-value, nonliteral, malformed-digest, or
unregistered matrix content. Its phase-aware range guard applies only to the
closure transition from no canonical phase node to the canonical closed phase
node. Stable later documentation changes do not reactivate the closure
allowlist.

## Evidence

| Acceptance row | Checked case IDs | Command |
| --- | --- | --- |
| Enumerate the intended plugin compatibility set. | `E01`, `L00`, `L01`, `L04`, `I01`, `I08`, `K01`, `K03`, `K05`, `K10`, `K12`, `K14`, `K16`, `K18`, `K20` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| Preserve one compatibility contract per cell. | `F01M`, `F01E`, `F01R`, `F01W`, `F01P`, `F01G`, `F01D`, `F01U`, `F01V`, `FREV` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| Keep cell identity unique and literal. | `K01`, `K03`, `K05`, `K10`, `K12`, `K14`, `K16`, `K18`, `K20` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| Close all references over the same set. | `R01M01`, `R01M02`, `R01M15`, `R01M16`, `R01M17`, `R01M18`, `R01M19`, `P01P01` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| Keep the prerequisite documentation-only. | `T00`, `T01`, `T10`, `T11`, `T12`, `W00`, `W01`, `W03`, `W07`, `W08`, `W10`, `W12`, `W17`, `W22`, `W23`, `W24`, `D01`, `D02`, `D03` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |

## Consequences

The lifecycle migration proposal can now select its frozen source inventory
target. That follow-up must preserve this matrix as a finite input unless a
separate proposal explicitly revises the membership.

Future plugin work must record exact compatibility values from checked client
and validator artifacts. It must not infer host builds, validator versions,
contract versions, or integrity digests from this planning matrix.
