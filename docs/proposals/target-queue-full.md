# Proposal Target Queue Full

Use this page after [target-queue.md](target-queue.md) when selecting,
comparing, or updating accepted proposal targets. This page is not a source for
implemented behavior; current behavior lives under `../reference/language/`.

## Read First

- Current implemented behavior:
  [../reference/language/README.md](../reference/language/README.md).
- Implementation workflow:
  [implementation-route.md](implementation-route.md).
- Promotion rules:
  [../document-status.md](../document-status.md).

## Target Boundaries

- Current self-hosting standard library behavior routes through
  [../reference/language/names-effects.md](../reference/language/names-effects.md).
- Current repair loop behavior routes through
  [../reference/language/holes.md](../reference/language/holes.md) and
  [../reference/language/diagnostics-json.md](../reference/language/diagnostics-json.md).
- Current predicate semantics route through
  [../reference/language/contracts.md](../reference/language/contracts.md) and
  [../reference/language/names-effects.md](../reference/language/names-effects.md).
- When a change touches both future targets, choose the one whose user-visible
  behavior changes first and leave the other target queued unless the queue
  also selects it.

## Selection Rule

- Choose the first target whose short proposal page still describes behavior
  absent from `../reference/language/`.
- Use [implementation-route.md](implementation-route.md) to compare only that
  target with current behavior before opening any full proposal record.
- If a target is already implemented, promote the behavior into
  `../reference/language/` and leave only remaining proposal work in the queue.
- If no accepted target has remaining work, use
  [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md) only
  for open design exploration, not as an accepted implementation target.

## Read When

- Use [first-slice-follow-ups.md](first-slice-follow-ups.md) only to confirm
  that no accepted first-slice target remains.
- Use [../reference/language/names-effects.md](../reference/language/names-effects.md)
  before adding standard library intrinsics or self-hosting library surface.
- Use [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md)
  only when the accepted target list does not match the implementation task.
- Use [implementation-route.md](implementation-route.md) after choosing a target
  and before editing the reference.

## Skip Unless Needed

- Do not open full proposal records before choosing a target above.
- Do not treat proposal text as implemented behavior unless
  `../reference/language/` also states it.
