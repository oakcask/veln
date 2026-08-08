---
role: implementation-record
authority: supporting
review-when: The record is superseded, its evidence links become invalid, or current documentation relies on it as authority.
---

# Toolchain User Cache Recovery

## Outcome

Reusable JVM class cache entries recover without executing or publishing an
invalid entry. Cache failures do not damage a complete entry published by a
concurrent invocation.

Current behavior is specified in
[Commands](../../specification/commands.md). This record preserves the
completion boundary and evidence; it is not the source of current behavior.

## Completed Acceptance Evidence

The `java::tests` unit tests provide deterministic fault injection at the JVM
cache filesystem boundaries:

- `invalid_entry_removal_failure_stops_java_and_later_revalidates_entry`
  preserves the invalid entry, proves that Java does not start, and proves that
  a later invocation revalidates and repairs the entry.
- `prepared_entry_validation_failure_cleans_up_and_allows_retry_and_reuse`
  proves cleanup after removal, later regeneration, and reuse.
- `publication_failure_leaves_selected_root_as_miss_and_allows_retry` proves
  that publication failure leaves no partial hit and permits retry below the
  selected root.
- `failed_writer_preserves_concurrently_published_winner_byte_for_byte` uses a
  controlled barrier and proves that a failed writer preserves the winner and
  its bytes.

The `toolchain_harness` test
`abandoned_jvm_cache_coordination_reaches_bounded_error_without_starting_java`
continues to provide process-level evidence for bounded coordination failure
before JVM startup.

## Scope Boundary

This work did not change cache-root selection, `VELN_CACHE_DIR`, cache keys,
entry layout, integrity manifests, successful publication behavior, eviction,
size policy, cache commands, manifest settings, dependency caches, or source
discovery.

## Verification

Run the focused checks through the repository wrapper:

```sh
bash scripts/agent-test -p veln-cli
bash scripts/agent-test -p veln-cli --test toolchain_harness
```
