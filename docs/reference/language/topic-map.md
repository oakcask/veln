# Language Topic Map

Use this page when you know the kind of behavior being changed but not the
smallest reference page to read. Start with the short page; open the matching
`*-full.md` file only when the short page names the detail you need.

## Source Surface

- Modules, items, expressions, literals, comments, tests, doctests, and grammar:
  [source-surface.md](source-surface.md).
- Type annotations, inference, assignment compatibility, and operators:
  [types.md](types.md).
- Names, stdio calls, prelude helpers, concurrency calls, and effects:
  [names-effects.md](names-effects.md).

## Contracts And Holes

- Start at [contracts-holes.md](contracts-holes.md) when a task touches both
  contracts and holes.
- Contract clauses, predicate validation, runtime obligations, blame, and
  result bindings: [contracts.md](contracts.md).
- Hole diagnostics, safe repair records, satisfy constraints, and candidate
  ranking: [holes.md](holes.md).

## Commands And Output

- Command gates, source discovery, entry selection, formatting, checking,
  running, and testing: [commands.md](commands.md).
- Choosing the command-specific machine-readable output page:
  [json-output.md](json-output.md).
- Diagnostic JSON envelope, related notes, spans, and stable details:
  [diagnostics-json.md](diagnostics-json.md).
- Run JSON records, output events, failures, and summaries:
  [run-json.md](run-json.md).
- Test JSON selection, case records, failures, errors, and summaries:
  [test-json.md](test-json.md).

## Runtime, Examples, And Rationale

- JVM execution behavior, values, calls, control flow, and host boundaries:
  [execution.md](execution.md).
- User-facing source examples: [examples.md](examples.md).
- Stability boundary and explicit non-goals: [overview.md](overview.md).
- Decision rationale after the behavior page is not enough:
  [source-decisions.md](source-decisions.md).
