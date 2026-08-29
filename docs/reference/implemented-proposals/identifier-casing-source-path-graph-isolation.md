---
role: implementation-record
authority: supporting
update-when: Source-path-derived module graph isolation, identifier-casing source-path examples, or dependency graph selection behavior changes.
---

# Identifier Casing Source Path Graph Isolation

## Outcome

Invalid source-path-derived module identities are absent from selected module
graph relationships. Current behavior is specified by
[Name Resolution](../../specification/name-resolution.md) and
[Check JSON And Diagnostics](../../specification/diagnostics-json.md).

The checked `identifier-casing-source-path-import-isolation-json` example
fixes that an invalid derived identity does not satisfy a selected local
import. The checked
`identifier-casing-source-path-duplicate-isolation-json` example fixes that
the invalid identity does not produce a duplicate source-module relationship.
The checked `identifier-casing-source-path-cycle-isolation-json` example and
the focused `veln-test` dependency graph test fix that imports written by a
source without a module identity do not contribute dependency graph edges.
Each checked command case also retains the existing source-path
`name.invalid_case` diagnostic and an independent diagnostic from an unrelated
valid selected module.

## Scope

This slice completes diagnostic-tolerant graph isolation for selected
`check` analysis and the shared test-selection dependency graph. It preserves
artifact command behavior: metrics reports, dependency-cycle policy results,
documentation generation, package snapshots, backend reachability, and
deferred language-service consumers remain outside this slice.
