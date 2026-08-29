---
role: routing
update-when: A specification topic route is added, moved, renamed, or no longer points to the smallest current behavior page.
---

# Language Topic Map

Use this page when you know the kind of behavior being changed but not the
smallest specification page to read. Choose the narrowest subject route and
stop when its authoritative page answers the task.

## Source Surface

- Modules, items, expressions, literals, comments, tests, doctests, and grammar:
  [source-surface.md](source-surface.md).
- Type annotations, inference, assignment compatibility, and operators:
  [types.md](types.md).
- Source name resolution and source identifier casing:
  [name-resolution.md](name-resolution.md).
- Compiler-provided source-less lookup registry validation:
  [source-less-lookup.md](source-less-lookup.md).
- Effect labels, stdio, file-system, network, time, process, and concurrency
  calls: [effects.md](effects.md).
- Compiler-known descriptor metadata and prelude helpers:
  [prelude-helpers.md](prelude-helpers.md).
- Editor lexical fallback, semantic token classes, and LSP full-token encoding:
  [editor-support.md](editor-support.md).

## Contracts And Holes

- Start at [contracts-holes.md](contracts-holes.md) when a task touches both
  contracts and holes.
- Contract clauses, predicate validation, runtime obligations, blame, and
  result bindings: [contracts.md](contracts.md).
- Hole diagnostics and satisfy constraints: [holes.md](holes.md).
- Advisory repair candidate records, concrete edits, application policy, the
  `repair` command gate, and future-work boundary:
  [repair-candidates.md](repair-candidates.md).
- Applying repair candidates, confirmation, override, verification, and
  rollback: [repair-application.md](repair-application.md).

## Commands And Output

- MCP stdio lifecycle, workspace project selection, saved diagnostics, saved
  definitions, tool declarations, and atomic refresh: [mcp.md](mcp.md).
- Command routes: [commands.md](commands.md).
- Shared analysis gates and source discovery:
  [command-analysis.md](command-analysis.md).
- Checking, formatting, documentation, running, and testing:
  [command-check.md](command-check.md), [command-fmt.md](command-fmt.md),
  [command-doc.md](command-doc.md), [command-run.md](command-run.md), and
  [command-test.md](command-test.md).
- Choosing the command-specific machine-readable output page:
  [json-output.md](json-output.md).
- Advisory source dependency metrics, dependency-cycle policy checks,
  experimental exact whole-body similarity, and metrics JSON:
  [command-metrics.md](command-metrics.md), then [metrics-json.md](metrics-json.md).
- Human diagnostics that need related notes or structured output coverage, and
  diagnostic JSON envelope, spans, related notes, and stable details:
  [diagnostics-json.md](diagnostics-json.md).
- Hole candidate JSON fields and application policy:
  [repair-candidates.md](repair-candidates.md), then
  [diagnostics-json.md](diagnostics-json.md).
- Run JSON records, output events, failures, and summaries:
  [run-json.md](run-json.md).
- Test JSON selection, case records, failures, errors, and summaries:
  [test-json.md](test-json.md).
- Repair JSON preview, apply, refusal, edit, verification, and summary records:
  [repair-json.md](repair-json.md), after
  [repair-application.md](repair-application.md) for write gates.

## Runtime, Examples, And Rationale

- Transport-independent package snapshot digest inputs, transcript, spelling,
  and fixed vectors: [package-snapshots.md](package-snapshots.md).
- Transport-independent package documentation catalogs, canonical result
  bytes, documentation digest, resource URI identity, gates, and disclosure
  boundaries: [package-documentation.md](package-documentation.md), then
  [package-documentation-full.md](package-documentation-full.md).
- Canonical package virtual-source URIs, listing, and exact resolution:
  [package-virtual-sources.md](package-virtual-sources.md).
- Explicit HTTP/2 frame, diagnostic, HPACK, and core modules:
  [http2.md](http2.md).
- JVM execution behavior, values, calls, control flow, and host boundaries:
  [execution.md](execution.md).
- User-facing source examples: [examples.md](examples.md).
- Stability boundary and explicit non-goals: [overview.md](overview.md).
- Decision rationale after the behavior page is not enough:
  [source-decisions.md](source-decisions.md).
