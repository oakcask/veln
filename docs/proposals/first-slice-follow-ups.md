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

- Use this page only to choose a target area.
- Open [first-slice-follow-ups-full.md](first-slice-follow-ups-full.md) only for
  the selected area's historical details.

## Accepted Targets

### Repair Loop

Use this target for remaining safe repair candidate generation, satisfy
predicate matching, candidate ranking, or repair JSON detail changes. Direct
top-level and nested `or` branch matching is already current behavior in
[../reference/language/holes.md](../reference/language/holes.md), including
candidate-specific discharge reasons; keep this target for broader discharge
beyond that implemented subset.

Before opening [the full repair-loop record](first-slice-follow-ups-full.md#repair-loop),
compare the task with [../reference/language/holes.md](../reference/language/holes.md)
and [../reference/language/diagnostics-json.md](../reference/language/diagnostics-json.md).

### Effects And Contracts

Use this target for contract predicate validation, static obligation
classification, transitive predicate implications, or effect propagation
changes.

Before opening
[the full effects and contracts record](first-slice-follow-ups-full.md#effects-and-contracts),
compare the task with [../reference/language/contracts.md](../reference/language/contracts.md)
and [../reference/language/names-effects.md](../reference/language/names-effects.md).

### Formatting

Use this target for changing the canonical formatter indentation rule. The
accepted target is to move from the current two-space indentation unit to a
tab-character indentation unit and to make every `match` arm one indentation
level deeper than the `match` expression line.

Before opening
[the full formatting record](first-slice-follow-ups-full.md#formatting),
compare the task with [../reference/language/commands.md](../reference/language/commands.md)
and [../reference/language/source-surface.md](../reference/language/source-surface.md).

## History

The full record also keeps empty historical categories for language and type
coverage, lowering and execution, and test discovery and events. Open those
sections only when auditing why no accepted follow-up is listed here.

## Skip Unless Needed

- Do not read the full record when the current language reference already
  answers the behavior question.
- Do not treat this proposal route as implemented behavior unless
  `../reference/language/` also states it.
