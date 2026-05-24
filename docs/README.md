# Veln Design Notes

This directory keeps the language-design discussion and durable decisions for
the experimental Veln implementation.

## Read First

- [reviews/2026-05-24-first-slice-gap-review.md](reviews/2026-05-24-first-slice-gap-review.md)
  is the current implementation gap review for the first slice.
- [phases/first-slice-implementation.md](phases/first-slice-implementation.md)
  is the current implementation memo for the first slice.
- [discussions/2026-05-24-agent-language-spec-wall.md](discussions/2026-05-24-agent-language-spec-wall.md)
  is the short entry point for the current design-wall discussion based on the
  agent-oriented language proposal.

## Conventions

- Put exploratory discussion logs in `discussions/`.
- Put implementation review findings and correction lists in `reviews/`.
- Keep stable language decisions short and promote them to a future `reference/`
  document when they stop changing.
- Prefer small, dated files so later agents can read only the relevant context.
- When a discussion accumulates decision results, keep the dated entry file as
  an index and move each result body into a companion detail directory.
