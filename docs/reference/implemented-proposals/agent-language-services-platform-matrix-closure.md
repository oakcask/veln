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
`claude-code/x86_64-unknown-linux-gnu`.

The same section declares the eleven compatibility field identities that future
plugin compatibility records must use. It also declares the thirteen
matrix-reference tuples that bind plugin requirements, Q21 evidence, Q22
totality, proposal completion, and lifecycle migration prerequisites to the
same closed matrix.

The documentation validator rejects hidden, displaced, duplicate, reordered,
malformed, or unregistered matrix content. Its phase-aware range guard applies
only to the closure transition from no canonical phase node to the canonical
closed phase node. Stable later documentation changes do not reactivate the
closure allowlist.

## Evidence

| Acceptance ID | Checked case IDs | Command |
| --- | --- | --- |
| `A01` | `L00`, `L01`, `L04`, `I01`, `I08`, `E01` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A02` | `F01M`, `F01D`, `F01U`, `F01V`, `FREV` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A03` | `K01`, `K03`, `K05`, `K10`, `K12`, `K14`, `K16`, `K18`, `K20` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A04` | `R01M01`, `R01M02`, `R01M15`, `R01M16`, `R01M17`, `R01M18`, `P01P01` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A05` | `I01`, `I02`, `I03`, `I04`, `I05`, `H01C01`, `H04C05`, `S02-S01` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A06` | `X-I09`, `X-U01`, `X-U03`, `X-U07`, `X-CRLF`, `X-PIPE` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A07` | `S00-S00`, `S00-S01`, `S01-S01`, `S01-S00`, `S03-S01` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A08` | `T00`, `T01`, `T10`, `T11`, `T12`, `T24`, `T25`, `T26`, `T27`, `T28` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A09` | `W00`, `W01`, `W03`, `W07`, `W08`, `W10`, `W12`, `W17`, `W22`, `W23`, `W24`, `W25`, `W26`, `W27` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A10` | `D01`, `D02`, `D03` | `node workflow-scripts/check-doc-frontmatter.mjs docs/proposals/README.md docs/proposals/agent-language-services.md docs/proposals/agent-language-services-lifecycle-migration.md docs/reference/implemented-proposals/README.md docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md` |

## Consequences

The lifecycle migration proposal can now select its frozen source inventory
target. That follow-up must preserve this matrix as a finite input unless a
separate proposal explicitly revises the membership.

Future plugin work must record exact compatibility values from checked client
and validator artifacts. It must not infer host builds, validator versions,
contract versions, or integrity digests from this planning matrix.
