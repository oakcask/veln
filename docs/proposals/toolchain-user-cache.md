---
role: proposal
review-when: The JVM cache failure-recovery contract, acceptance evidence, or implementation status changes.
---

# Toolchain User Cache Recovery

## Summary

Complete the remaining failure-recovery and concurrent-preparation guarantees
for reusable JVM class cache entries. Cache-root selection, `VELN_CACHE_DIR`,
command validation timing, entry integrity checks, ordinary corruption repair,
and successful concurrent publication are current behavior specified in
[Commands](../specification/commands.md).

## Remaining Behavior

An invocation must not execute an invalid published entry. If it cannot remove
an invalid entry, it must report a focused cache error and leave the entry
subject to full revalidation by later invocations.

If invalid-entry removal succeeds but regeneration, validation, or publication
fails, the invocation must publish no partial replacement. A later invocation
must observe a miss and be able to retry generation.

A writer failure must not delete, replace, or invalidate a complete entry that
another invocation published for the same key. If no invocation can obtain a
valid winner, each affected command must fail without starting the JVM.

If a process stops while coordinating or preparing an entry, a later
invocation must recover or report a bounded cache-coordination error. It must
not wait without a bound. The coordination representation, waiting strategy,
and bound are not part of the external contract.

## Acceptance Model

The evidence column names planned evidence. It does not describe tests that
already pass.

| Case | Injected condition | Required result | Planned primary evidence |
| --- | --- | --- | --- |
| Removal failure | A corrupt published entry cannot be removed | The command reports a cache error, does not start the JVM, and later invocations revalidate the entry | Fault-injected JVM cache unit test |
| Regeneration failure | Removal succeeds but preparation or validation fails | No partial entry is published, and a later invocation can regenerate | Fault-injected JVM cache unit test |
| Publication failure | Preparing or publishing below the selected root fails | No partial entry becomes a hit and no fallback is used | Fault-injected JVM cache unit test |
| Failed writer isolation | One writer fails after another publishes a valid entry | The valid winner remains byte-for-byte valid and reusable | Barrier-controlled JVM cache concurrency test |
| Abandoned coordination | A process stops while coordinating an entry | A later invocation recovers or reaches a bounded error without executing remnants | Process-level JVM cache integration test |

## Non-Goals

- Do not change cache-root selection or `VELN_CACHE_DIR` semantics.
- Do not redesign cache keys, entry layout, integrity manifests, or successful
  publication behavior.
- Do not add eviction, size policy, cache commands, manifest settings, or
  dependency caches.
- Do not define a source-discovery exclusion for the cache root.

## Verification

Implementation must add the fault-injected and process-level evidence named in
the acceptance model. Run the focused checks through the repository wrapper:

```sh
bash scripts/agent-test -p veln-cli
bash scripts/agent-test -p veln-cli --test toolchain_harness
```

## Completion Boundary

This proposal is complete when every acceptance row has passing evidence and
the current command specification describes the resulting recovery behavior.
Move the completed record to `../reference/implemented-proposals/` and remove
it from the proposal catalog.
