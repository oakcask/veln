---
role: routing
update-when: A specification topic route is added, moved, renamed, or no longer points to the smallest current behavior page.
---

# Language Topic Map

Use this page when you know the kind of behavior being changed but not the
smallest specification page to read. Start with the short page; open the matching
`*-full.md` file only when the short page names the detail you need.

## Source Surface

- Modules, items, expressions, literals, comments, tests, doctests, and grammar:
  [source-surface.md](source-surface.md).
- Type annotations, inference, assignment compatibility, and operators:
  [types.md](types.md).
- Names, stdio calls, prelude helpers, concurrency calls, and effects:
  [names-effects.md](names-effects.md).
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

- Command gates, source discovery, entry selection, formatting, checking,
  running, and testing: [commands.md](commands.md).
- Choosing the command-specific machine-readable output page:
  [json-output.md](json-output.md).
- Advisory source dependency metrics, dependency-cycle policy checks,
  experimental exact whole-body similarity, and metrics JSON:
  [commands.md](commands.md), then [metrics-json.md](metrics-json.md).
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
