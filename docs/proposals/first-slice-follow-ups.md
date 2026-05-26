# First-Slice Follow-Up Targets

Status: promoted
Implementation: promoted to reference: no accepted first-slice follow-up
targets currently remain.

This is the routing page for accepted first-slice targets and their promotion
history. Use it to confirm whether any first-slice follow-up target remains
before opening the full follow-up record.

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

- No accepted first-slice follow-up targets currently remain.

## History

### Effects And Contracts

The richer predicate semantics target has been promoted to current behavior.
Use [../reference/language/contracts.md](../reference/language/contracts.md)
for contract predicate validation, static obligation classification,
transitive predicate implications, runtime obligations, and result bindings.
Use [../reference/language/names-effects.md](../reference/language/names-effects.md)
for effect propagation and compiler-known calls.

### Repair Loop

The broader repair discharge target has been promoted to current behavior. Use
[../reference/language/holes.md](../reference/language/holes.md) and
[../reference/language/diagnostics-json.md](../reference/language/diagnostics-json.md)
for the implemented repair candidate and JSON behavior. Open
[the full repair-loop record](first-slice-follow-ups-full.md#repair-loop) only
for proposal history.

### Formatting

The formatter indentation target has been promoted to current behavior. Use
[../reference/language/commands.md](../reference/language/commands.md) for the
implemented `fmt` route and
[../reference/language/commands-full.md#veln-fmt-path](../reference/language/commands-full.md#veln-fmt-path)
for the canonical indentation rule. Open
[the full formatting record](first-slice-follow-ups-full.md#formatting) only
for proposal history.

The full record also keeps empty historical categories for language and type
coverage, lowering and execution, and test discovery and events. Open those
sections only when auditing why no accepted follow-up is listed here.

## Skip Unless Needed

- Do not read the full record when the current language reference already
  answers the behavior question.
- Do not treat this proposal route as implemented behavior unless
  `../reference/language/` also states it.
