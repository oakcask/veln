# Names And Effects

This is the routing page for implemented name resolution, effect checking, and
compiler-known calls.

## Read First

- Name-resolution namespaces, shadowing, duplicate checks, module ownership,
  and metadata drift: [names-effects-full.md](names-effects-full.md).
- Effect labels and stdio calls:
  [names-effects-full.md](names-effects-full.md#stdio-calls).
- Concurrency calls, executable reachability, and effect provenance:
  [names-effects-full.md](names-effects-full.md#concurrency-calls).
- Prelude helpers and their value semantics:
  [names-effects-full.md](names-effects-full.md#prelude-helpers).

## Read When

- Updating `name.*`, `module.*`, or `effect.*` diagnostics.
- Changing compiler-known calls, reachability, prelude helpers, or effect
  inference.
- Deciding whether a behavior belongs in the implemented reference or remains
  proposal rationale.

## Skip Unless Needed

- Do not open source-decision history before the implemented behavior in
  [names-effects-full.md](names-effects-full.md) answers the question.
- Use [diagnostics-json.md](diagnostics-json.md) only when the machine-readable
  shape of a diagnostic also changes.
