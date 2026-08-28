---
role: implementation-record
authority: supporting
update-when: Source-path-derived module identity graph-isolation behavior, executable identifier-casing graph examples, or current specification authority for this record changes.
---

# Identifier Casing Source Path Graph Isolation

## Outcome

Diagnostic-tolerant analysis now keeps sources with invalid
source-path-derived module identities out of local import resolution,
duplicate source-path detection, and test dependency graph edges. The invalid
source still reports the existing `name.invalid_case` diagnostic for the
source path segment. Valid unrelated sources in the same selected project
continue to produce their ordinary diagnostics or test analysis results.

Current behavior is specified by
[Name Resolution](../../specification/name-resolution.md),
[Check JSON And Diagnostics](../../specification/diagnostics-json.md), and
[Test JSON](../../specification/test-json.md).

## Evidence

The checked
`identifier-casing-source-path-import-isolation-json` example fixes local
import resolution isolation. The checked
`identifier-casing-source-path-duplicate-isolation-json` example fixes
duplicate source-path isolation. The checked test
`identifier-casing-source-path-graph-isolation-json` example and focused
`veln-test` unit coverage fix that imports declared by a source without a
normal module identity do not add dependency graph edges before dependency
closure is computed. The checked metrics
`identifier-casing-source-path-cycle-isolation-json` example and focused
`veln-metrics` unit coverage fix that invalid identities do not become metrics
graph nodes or dependency-cycle policy violations.

## Completion

This slice completes diagnostic-tolerant graph isolation for source-path
derived module identity casing, including the metrics dependency graph and
dependency-cycle policy boundary. It does not complete artifact consumers,
deferred language-service consumers, qualified-use segment casing, recovery
navigation, repair rename, rename conflict prediction, or MCP rename mapping.
