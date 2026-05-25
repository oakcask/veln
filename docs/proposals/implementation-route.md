# Proposal Implementation Route

Use this page after choosing to turn proposal text into implemented behavior.
Start from [README.md](README.md) when you only need to find the relevant
proposal.

## Choose A Target

- Choose from [target-queue.md](target-queue.md) before opening broader
  design-wall material.
- Remaining first-slice implementation targets are tracked in
  [first-slice-follow-ups.md](first-slice-follow-ups.md), with full detail in
  [first-slice-follow-ups-full.md](first-slice-follow-ups-full.md).
- Use design-wall material only when [target-queue.md](target-queue.md) has no
  accepted target for the task.

## Compare And Promote

- Compare the target with current behavior in
  [../reference/language/README.md](../reference/language/README.md) so the
  implementation changes only the missing behavior.
- Use [../reference/language/topic-map.md](../reference/language/topic-map.md)
  to choose the smallest reference page to update after the code changes.
- Use [../reviews/first-slice-gap-review.md](../reviews/first-slice-gap-review.md)
  for evidence about known gaps before treating a proposal as complete.
- After implementation, promote the resulting behavior into
  `../reference/language/` and keep proposal text only for remaining incomplete
  or historical context.
- Use [../document-status.md](../document-status.md) before promoting,
  superseding, or rejecting proposal text.

## Exit Checklist

- The changed behavior is documented under `../reference/language/`.
- Remaining proposal text still describes only absent or incomplete behavior.
- Links from [target-queue.md](target-queue.md) still route to a live accepted
  target, or the target has been removed from the accepted queue.

## Skip Unless Needed

- Do not read design-wall material before the accepted target queue fails to
  route the task.
- Do not edit full proposal history merely to mark implementation status; keep
  current behavior in `../reference/language/` and leave remaining proposal work
  in short route pages.
