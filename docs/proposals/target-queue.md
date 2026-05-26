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

- Self-hosting standard library: start with
  [self-hosting-standard-library.md](self-hosting-standard-library.md).
- Formatter indentation: start with
  [first-slice-follow-ups.md#formatting](first-slice-follow-ups.md#formatting).
- Broader repair discharge: start with
  [first-slice-follow-ups.md#repair-loop](first-slice-follow-ups.md#repair-loop).
- Richer predicate semantics: start with
  [first-slice-follow-ups.md#effects-and-contracts](first-slice-follow-ups.md#effects-and-contracts).

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
- Use [self-hosting-standard-library.md](self-hosting-standard-library.md) when
  the task adds `fs`, `process`, collection, string, or descriptor-backed
  standard symbols.

## Skip Unless Needed

- Do not treat proposal text as implemented behavior unless
  `../reference/language/` also states it.
