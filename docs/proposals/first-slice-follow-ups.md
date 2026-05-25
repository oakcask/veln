# First-Slice Follow-Up Targets

Status: accepted-proposal
Implementation: partially implemented

This is the routing page for accepted first-slice targets that are not fully
implemented in the current workspace. Use it to choose a target area before
opening the full follow-up record.

## Read First

- [../reference/language/README.md](../reference/language/README.md): current
  implemented behavior.
- [../reviews/first-slice-gap-review.md](../reviews/first-slice-gap-review.md):
  review routing and historical gap evidence.
- [../document-status.md](../document-status.md): promotion boundary when a
  follow-up becomes implemented behavior.

## Read When

- Use this page to choose a target area.
- Open [first-slice-follow-ups-full.md](first-slice-follow-ups-full.md) only for
  the selected area's historical details.

## Accepted Targets

### Repair Loop

Remaining work is broader repair discharge beyond normalized direct and
`require`-matched cases. Read
[the full repair-loop record](first-slice-follow-ups-full.md#repair-loop)
only after confirming the missing behavior is not already covered by
[../reference/language/holes.md](../reference/language/holes.md) and
[../reference/language/diagnostics-json.md](../reference/language/diagnostics-json.md).

Use this target for safe repair candidate generation, satisfy predicate
matching, candidate ranking, or repair JSON detail changes.

### Effects And Contracts

Richer predicate semantics remain follow-up work beyond the implemented
contract subset. Numeric literal-bound implication through equality aliases is
now reference behavior; remaining work should stay outside that implemented
slice. Read
[the full effects and contracts record](first-slice-follow-ups-full.md#effects-and-contracts)
only after confirming the missing behavior is not already covered by
[../reference/language/contracts.md](../reference/language/contracts.md) and
[../reference/language/names-effects.md](../reference/language/names-effects.md).

Use this target for contract predicate validation, static obligation
classification, transitive predicate implications, or effect propagation
changes.

## History

The full record also keeps empty historical categories for language and type
coverage, formatting, lowering and execution, and test discovery and events.
Open those sections only when auditing why no accepted follow-up is listed
here.

## Skip Unless Needed

- Do not read the full record when the current language reference already
  answers the behavior question.
- Do not treat this proposal route as implemented behavior unless
  `../reference/language/` also states it.
