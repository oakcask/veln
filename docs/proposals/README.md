# Proposals

This directory keeps design targets that are accepted or being explored but are
not fully implemented in the current workspace.

## Read First

- [target-queue.md](target-queue.md) is the shortest route for choosing an
  accepted implementation target.
- [implementation-route.md](implementation-route.md) explains how to compare a
  chosen target with current behavior and promote implemented behavior into
  the reference.
- [../document-status.md](../document-status.md) defines promotion and
  supersession labels when proposal text moves into reference material.

## Implementation Route

- Choose one target from [target-queue.md](target-queue.md).
- Read [implementation-route.md](implementation-route.md) before opening full
  proposal history.
- Compare the selected proposal with current behavior through
  [../reference/language/topic-map.md](../reference/language/topic-map.md).
- After implementation, promote the resulting behavior into
  `../reference/language/` and leave only remaining proposal work here.
- Keep [target-queue.md](target-queue.md) as the accepted-target source; use
  design-wall pages only after the queue has no matching implementation target.

## Read When

- Selecting an accepted implementation target:
  [target-queue.md](target-queue.md).
- Turning a proposal into reference behavior:
  [implementation-route.md](implementation-route.md).
- Checking design-wall decisions that are accepted or still open:
  [agent-language-spec-wall/README.md](agent-language-spec-wall/README.md).
- Checking review evidence about gaps or completion claims:
  [../reviews/first-slice-gap-review.md](../reviews/first-slice-gap-review.md).
- Moving, superseding, or rejecting proposal text:
  [../document-status.md](../document-status.md).

## Skip Unless Needed

- Use `../reference/` when you need current implemented behavior.
- Use `../phases/` only for implementation order or completion notes.
- Use `agent-language-spec-wall/` only after the accepted target route does not
  answer the planning question.
- Do not open `*-full.md` proposal records until a short proposal page names
  the section needed for the task.
