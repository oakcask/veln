---
role: implementation-record
authority: supporting
update-when: The closed agent-language-services client-platform matrix, matrix reference registry, validator evidence, or lifecycle prerequisite route is superseded.
---

# Agent Language Services Platform Matrix Closure

## Summary

The agent-language-services proposal now has one closed client-platform
membership table. The table contains exactly `codex/x86_64-unknown-linux-gnu`
followed by `claude-code/x86_64-unknown-linux-gnu`. The proposal also declares
the eleven compatibility fields as exact literal values in each row.

This record closes only the finite documentation contract. It does not claim
that either client-platform cell is supported, installable, or host-validated.

## Completed Contract

The canonical source is
[Closed Client-Platform Matrix](../../proposals/agent-language-services.md#closed-client-platform-matrix).
The matrix section owns the row-count paragraph, phase identity, compatibility
table, and matrix-reference registry. All plugin, Q21, Q22, completion, and
lifecycle-prerequisite references route to that registry.

The documentation validator
`workflow-scripts/check-agent-language-platform-matrix.mjs` checks the
repository documents and the phase-aware range guard. The guard applies to the
closure transition from a base without the phase identity to a head with the
closed matrix. It retires for later `present` to `present` documentation
changes.

## Evidence

| Acceptance ID | Checked case IDs | Command |
| --- | --- | --- |
| `A01` | `L00`, `L01`, `E01` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A02` | `F01`, `F02`, `F03`, `F04`, `F05` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A03` | `K01`, `K02`, `K03`, `K04`, `K05`, `K06` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A04` | `R01`, `R02`, `R03`, `R04`, `P01`, `P02` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A05` | `I01`, `I02`, `H01`, `H02`, `S02`, `S03`, `S04` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A06` | `X-U01`, `X-U02`, `X-CRLF`, `X-PIPE` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A07` | `S00-S01`, `S01-S01`, `S01-S00`, `S02-S01` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A08` | `T00`, `T01`, `T10`, `T25`, `T26`, `T27`, `T28` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A09` | `W00`, `W01`, `W07`, `W08`, `W10`, `W17`, `W18`, `W27`, `E02` | `node --test workflow-scripts/check-agent-language-platform-matrix.test.mjs` |
| `A10` | `D01`, `D02`, `D03` | `node workflow-scripts/check-doc-frontmatter.mjs docs/reference/implemented-proposals/agent-language-services-platform-matrix-closure.md docs/proposals/README.md docs/proposals/agent-language-services.md docs/proposals/agent-language-services-lifecycle-migration.md docs/reference/implemented-proposals/README.md` |

## Consequences

The lifecycle migration is no longer blocked by an unnamed platform universe.
The next selectable agent-language-services target is the frozen source
inventory. That target remains documentation-only and must not implement MCP
fixtures, plugin manifests, smoke tests, or language behavior.
