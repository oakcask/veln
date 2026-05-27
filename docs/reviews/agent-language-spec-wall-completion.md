# Agent-Language Spec Wall Completion Review

Status: complete for accepted implementation targets.

This review covers the current target selected by `prompts/TARGET.md`: the
open design-wall material under `../proposals/agent-language-spec-wall/`.
The directory remains proposal material, but no accepted implementation target
is left to carry forward from it.

## Completion Check

- `../proposals/target-queue.md` reports no accepted targets.
- `../proposals/first-slice-follow-ups.md` reports no accepted first-slice
  follow-up targets.
- `../proposals/agent-language-spec-wall/open-questions.md` now routes the
  resolved first-slice questions to source-decision records and current
  language reference pages.
- Current implemented behavior routes through `../reference/language/`, with
  short topic pages for source surface, names and effects, contracts, holes,
  commands, JSON output, execution, and editor support.
- The remaining design-wall pages are explicitly exploratory. New work from
  those pages must first become a selected proposal target before changing
  current behavior.

## Residual Scope

The design-wall directory still holds broad thesis and historical question
inventory. That is intentional proposal history, not incomplete accepted work.
Open details inside implemented source-decision records stay bounded by the
current reference pages; they do not reopen the target queue unless promoted
as a new accepted target.

The prelude complexity decision is an intentional non-promise in the current
reference. `../reference/language/names-effects.md` documents value semantics,
source-order traversal, `Result` short-circuiting, and the absence of
asymptotic complexity guarantees, so the `accepted-proposal` status on that
record is not a blocking implementation target.

## Verification

- `rg` over `../proposals/` and `../reference/source-decisions/` for accepted
  and open status labels.
- `cargo test -p veln-test doctest`
- `cargo test -p veln-cli --test check_json negative_doctest`
- `cargo test -p veln-cli --test toolchain_harness`
