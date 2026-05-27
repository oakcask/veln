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
- This means there is no proposal completion condition to implement from this
  queue. Do not promote open design-wall material into implementation work
  unless a later queue update selects it as an accepted target.

## Selection Rule

- Choose the first target whose short proposal page still describes behavior
  absent from `../reference/language/`.
- If the accepted target list is empty, stop target selection here; the
  implementation route has no selected proposal to compare or promote.
- Compare the selected target through
  [implementation-route.md](implementation-route.md).
- Use [target-queue-full.md](target-queue-full.md) for target boundaries,
  queue updates, and the no-target fallback.

## Read When

- Use [first-slice-follow-ups.md](first-slice-follow-ups.md) only to confirm
  that no accepted first-slice target remains.
- Use [../reference/language/names-effects.md](../reference/language/names-effects.md)
  before adding `fs`, `process`, collection, string, or descriptor-backed
  standard symbols.

## Skip Unless Needed

- Do not treat proposal text as implemented behavior unless
  `../reference/language/` also states it.
