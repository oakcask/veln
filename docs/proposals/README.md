# Proposals

This directory keeps accepted or open design targets that are not fully
implemented.

## Read First

- Accepted targets: [target-queue.md](target-queue.md).
- Promotion route: [implementation-route.md](implementation-route.md).
- Status labels: [../document-status.md](../document-status.md).

## Implementation Route

- Choose one target from [target-queue.md](target-queue.md).
- Use [implementation-route.md](implementation-route.md) to compare only the
  chosen target with current behavior and decide what moves into
  `../reference/language/`.
- Open design-wall pages only when the queue has no matching target.

## Read When

- Use [first-slice-follow-ups.md](first-slice-follow-ups.md) only to confirm
  that no accepted first-slice follow-up target remains.
- Use [self-hosting-standard-library.md](self-hosting-standard-library.md) only
  for future self-hosting standard library questions whose behavior is absent
  from the current reference.
- Use [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md)
  for design-wall material that is still open or only partially represented.
- Use [../reviews/README.md](../reviews/README.md) when checking gap evidence
  before changing target status.

## Update When

- A target is implemented and the resulting behavior has been documented under
  `../reference/language/`.
- A target is found to be already implemented by the current reference and only
  remaining proposal work should stay queued.
- No accepted target remains and design-wall material should be left as
  exploration instead of being selected into the queue.

## Skip Unless Needed

- Use `../reference/` when you need current implemented behavior.
- Use `../phases/` only for implementation order.
- Do not open `*-full.md` proposal records until a short proposal page names
  the section needed for the task.
