# Proposals

This directory keeps accepted or open design targets and short promotion notes
for targets that have moved into the current language reference.

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

- Use [first-slice-follow-ups.md](first-slice-follow-ups.md) for accepted
  first-slice follow-up targets.
- Use [self-hosting-standard-library.md](self-hosting-standard-library.md) for
  accepted standard library and compiler-known intrinsic work needed for
  eventual self-hosting.
- Editor semantic highlighting has moved into current behavior; use
  [../reference/language/editor-support.md](../reference/language/editor-support.md)
  first, and open
  [editor-semantic-highlighting.md](editor-semantic-highlighting.md) only for
  proposal history.
- Use [toolchain-test-harness.md](toolchain-test-harness.md) for the open
  proposal to standardize command-line integration test cases.
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
  exploration instead of being promoted into the queue.

## Skip Unless Needed

- Use `../reference/` when you need current implemented behavior.
- Use `../phases/` only for implementation order.
- Do not open `*-full.md` proposal records until a short proposal page names
  the section needed for the task.
