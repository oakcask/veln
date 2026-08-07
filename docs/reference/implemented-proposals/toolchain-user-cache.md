---
role: implementation-record
authority: supporting
review-when: The cache recovery evidence, links, or completion boundary is superseded or becomes invalid.
---

# Toolchain User Cache Recovery

Current cache behavior is specified in
[Commands](../../specification/commands.md). This record preserves the bounded
completion evidence for cache failure recovery and concurrent failed-writer
isolation.

## Completed Boundary

The implementation retained cache-root selection, cache keys, entry layout,
integrity metadata, and the successful publication path. It added injectable
test boundaries for invalid-entry removal, preparation validation, and
publication. Failed preparation and publication remove private preparation
remnants before returning an error.

## Acceptance Evidence

| Case | Required state after failure | Executable evidence |
| --- | --- | --- |
| Removal failure | The invalid entry remains unpublished as valid and is fully revalidated by a later invocation | `removal_failure_retains_invalid_entry_for_later_revalidation` |
| Regeneration failure | No replacement or preparation remnant remains, and a later invocation can regenerate the missing entry | `regeneration_failure_leaves_no_published_or_partial_entry` |
| Publication failure | No partial entry becomes a hit, and a later invocation retries at the selected root | `publication_failure_leaves_a_miss_for_later_retry` |
| Failed writer isolation | A complete entry published by another writer remains byte-for-byte unchanged, valid, and reusable | `failed_writer_preserves_complete_entry_published_by_another_writer` |

All four cases run in the `veln-cli` unit test target. The failed-writer case
uses a barrier-controlled interleaving so the successful writer publishes
before the injected publication failure resumes. The command harness also
retains process-level evidence that an abandoned cache coordinator reaches a
bounded pre-JVM error without altering a complete entry.

## Verification

The focused checks are:

```sh
bash scripts/agent-test -p veln-cli
bash scripts/agent-test -p veln-cli --test toolchain_harness
```

No `examples/specification` fixture represents this contract. The material
failures require injected filesystem boundaries and controlled writer
interleavings, so the focused tests are the executable specification evidence.

## Non-Goals Preserved

This work did not add eviction, cache-size policy, cache commands, manifest
settings, dependency caches, or a source-discovery exclusion for the cache
root.
