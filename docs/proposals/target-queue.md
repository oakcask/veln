# Proposal Target Queue

Use this page when selecting one accepted proposal to implement. Return to
[README.md](README.md) when you only need the proposal directory map. Use
[target-queue-full.md](target-queue-full.md) only after choosing a target or
when updating this queue.

## Read First

- Current implemented behavior:
  [../reference/language/README.md](../reference/language/README.md).
- Implementation workflow:
  [implementation-route.md](implementation-route.md).
- Promotion rules:
  [../document-status.md](../document-status.md).

## Accepted Targets

- No accepted targets currently remain.

## Promoted Targets

- Richer predicate semantics has moved into current behavior; start with
  [../reference/language/contracts.md](../reference/language/contracts.md)
  for contract predicate validation, static obligation classification,
  runtime obligations, and result bindings, and
  [../reference/language/names-effects.md](../reference/language/names-effects.md)
  for effect propagation and compiler-known calls.
- Broader repair discharge has moved into current behavior; start with
  [../reference/language/holes.md](../reference/language/holes.md) and
  [../reference/language/diagnostics-json.md](../reference/language/diagnostics-json.md).
- Self-hosting standard library has moved into current behavior; start with
  [../reference/language/names-effects.md](../reference/language/names-effects.md)
  and use [self-hosting-standard-library.md](self-hosting-standard-library.md)
  only for proposal history.

## Selection Rule

- Choose the first target whose short proposal page still describes behavior
  absent from `../reference/language/`.
- Compare the selected target through
  [implementation-route.md](implementation-route.md).
- Use [target-queue-full.md](target-queue-full.md) for target boundaries,
  queue updates, and the no-target fallback.

## Read When

- Use [first-slice-follow-ups.md](first-slice-follow-ups.md) for the accepted
  first-slice target area before opening the full follow-up record.
- Use [../reference/language/names-effects.md](../reference/language/names-effects.md)
  before adding `fs`, `process`, collection, string, or descriptor-backed
  standard symbols.

## Skip Unless Needed

- Do not treat proposal text as implemented behavior unless
  `../reference/language/` also states it.
