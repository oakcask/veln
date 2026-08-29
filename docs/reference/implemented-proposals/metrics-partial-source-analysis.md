---
role: implementation-record
update-when: The implemented metrics behavior for invalid source-path module identities, partial report completeness, source diagnostics, policy evaluation, or baseline handling changes.
---

# Metrics Partial Source Analysis

## Summary

`veln metrics` returns useful path-based measurements and the valid portion of
its module graph when project-owned source fails only with source-path-derived
module identity casing diagnostics. The command retains those diagnostics,
marks the report incomplete, and exits non-zero.

Current behavior is specified in [Metrics JSON](../../specification/metrics-json.md)
and routed from [Commands](../../specification/commands.md).

## Completion Evidence

The metrics checked examples cover:

- advisory partial reports with retained diagnostics, excluded sources,
  retained path-based ABC subjects, and non-zero exit;
- checked hidden cycles where the retained graph has no known violation and
  the result is incomplete rather than pass;
- checked known cycles where retained-graph policy violations take precedence;
- explicit invalid source selection;
- mixed source errors that keep the ordinary diagnostic envelope;
- baseline write refusal without modifying the requested file;
- baseline checks that classify currently invalid module identities as
  excluded baseline subjects instead of stale subjects.

Focused `veln-metrics` unit tests cover graph-node exclusion, import exclusion
from identityless sources, path-based subject retention, policy precedence,
baseline subject classification, and mixed-error fallback.
