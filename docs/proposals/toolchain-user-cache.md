---
role: proposal
review-when: The JVM cache failure-recovery contract, acceptance evidence, or implementation status changes.
---

# Toolchain User Cache Recovery

## Summary

Complete the remaining publication-failure and failed-writer-isolation
guarantees for reusable JVM class cache entries. Cache-root selection,
`VELN_CACHE_DIR`, command validation timing, entry integrity checks, ordinary
corruption repair, invalid-entry removal failure, regeneration failure, and
successful concurrent publication are current behavior specified in
[Commands](../specification/commands.md).

## Remaining Behavior

If publication below the selected cache root fails, an invocation must not use
a partial entry or fall back to another root.

A writer failure must not delete, replace, or invalidate a complete entry that
another invocation published for the same key. If no invocation can obtain a
valid winner, each affected command must fail without starting the JVM.

## Acceptance Model

The evidence column names planned evidence. It does not describe tests that
already pass.

| Case | Injected condition | Required result | Planned primary evidence |
| --- | --- | --- | --- |
| Publication failure | Preparing or publishing below the selected root fails | No partial entry becomes a hit and no fallback is used | Fault-injected JVM cache unit test |
| Failed writer isolation | One writer fails after another publishes a valid entry | The valid winner remains byte-for-byte valid and reusable | Barrier-controlled JVM cache concurrency test |

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
