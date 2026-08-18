---
role: proposal
update-when: The agent-language-services plugin clients, supported platforms, compatibility fields, or lifecycle-migration source-universe prerequisite changes.
---

# Agent Language Services Platform Matrix Closure

## Summary

Replace the undefined agent-plugin platform set with a literal closed matrix.
The lifecycle migration cannot freeze every client-platform cell while the
umbrella proposal names clients but quantifies over an unnamed set of
“supported platforms.”

This is a documentation-contract prerequisite. It does not implement a plugin,
run host validation, or claim that a client-platform cell already passes.

## Selection State

This proposal is ready. Complete it before selecting the frozen source
inventory PR from
[Agent Language Services Lifecycle Migration](agent-language-services-lifecycle-migration.md).

## Scope

Add one literal client-platform table to
`agent-language-services.md`. Each data row declares one exact client and
platform pair. Each row also names the compatibility fields that the future
`compatibility.toml` entry must pin:

- client identifier;
- platform identifier;
- host build;
- manifest-schema revision;
- validator version and integrity digest; and
- required Veln, MCP, LSP, language-service, and reference-schema contracts.

The initial table contains exactly these two ordered client-platform keys:

1. `codex/x86_64-unknown-linux-gnu`
2. `claude-code/x86_64-unknown-linux-gnu`

The closure PR may not select a different platform, add a third row, reorder
the keys, or use a range, wildcard, “all supported platforms,” or an unnamed
future row. A separate proposal must add another client or platform after this
closure completes.

Every compatibility field is a nonempty literal. Host builds, schema and
contract revisions, and validator versions use one exact value, not a range or
placeholder. An integrity digest is exactly 64 lowercase hexadecimal digits.

Update every plugin requirement, Q21 evidence row, Q22 totality row, and
completion rule that quantifies over client-platform cells so it routes to the
literal table. A requirement may say “every row in the closed table”; it may
not introduce another implicit platform set.

## Checked Matrix Contract

Add a documentation validator that extracts the literal table independently
from the later frozen-source inventory. The validator records the ordered
client-platform keys and exact row count. It rejects:

- a missing or duplicate client-platform key;
- an empty client or platform identifier;
- a range, wildcard, placeholder, or catch-all row;
- a missing compatibility field;
- an empty, ranged, wildcard, or placeholder compatibility value;
- an integrity digest that is not exactly 64 lowercase hexadecimal digits;
- a plugin, Q21, Q22, or completion reference to an unnamed platform set; and
- a checked row-count value that differs from the table.

The validator runs through the existing documentation-validation workflow.
Its failure output names the invalid row or reference, tells the maintainer to
enumerate or restore the exact cell, and explains that the lifecycle inventory
cannot prove finite coverage otherwise.

The closure diff guard is active only when the base revision lacks the closed
matrix and the head revision adds the exact two-row table. It retires after
that transition merges. A range test proves that an unrelated documentation
change after closure does not inherit the closure allowlist. Separate tests
reject a protected-path rename and a Git type change during the closure PR.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Enumerate the intended plugin compatibility set. | One literal table contains `codex/x86_64-unknown-linux-gnu` followed by `claude-code/x86_64-unknown-linux-gnu` and no other or implicit membership. | Independent expected-key fixture and exact row count of two. |
| Preserve one compatibility contract per cell. | Every row contains nonempty exact client, platform, host build, manifest-schema revision, validator version and digest, and all required Veln contract fields. | Per-field missing, empty, range, wildcard, placeholder, and malformed-digest mutations. |
| Keep cell identity unique and literal. | Empty, duplicate, ranged, wildcard, placeholder, and catch-all client-platform keys fail. | Injected rejection cases for each invalid key class. |
| Close all references over the same set. | Plugin prose, Q21, Q22, and the completion rule refer only to the literal table. | Reference scan with one injected unnamed-platform phrase. |
| Keep the prerequisite documentation-only. | The matrix-addition transition does not add plugin artifacts, executable MCP cases, harness changes, or semantic baselines, and its allowlist retires after merge. | Phase-aware diff-scope cases for the matrix transition, a later unrelated docs change, a protected-path rename, and a Git type change. |

## Non-Goals

- Choosing or implementing the future plugin packaging mechanism.
- Claiming support for a client-platform cell before its pinned validation
  passes.
- Adding `compatibility.toml`, client manifests, MCP or LSP smoke tests, or an
  installer.
- Freezing the broader agent-language-services source inventory.
- Adding another client or platform after the exact two-cell set closes.

## Completion Rule

This proposal completes only when all five acceptance rows pass and the
umbrella proposal contains no unbound supported-platform phrase. Move the
completed record out of `docs/proposals/` before selecting the lifecycle
migration's frozen-source-inventory PR.
