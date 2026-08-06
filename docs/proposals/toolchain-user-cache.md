---
role: proposal
review-when: The toolchain cache location, override contract, acceptance evidence, or implementation status changes.
---

# Toolchain User Cache

## Summary

Store reusable toolchain cache data below the host operating system's standard
per-user cache directory instead of below a package or working directory.

The `VELN_CACHE_DIR` environment variable overrides the complete cache root.
This proposal keeps the external contract limited to the default location and
that override.

## Motivation

The current JVM class cache is written below `target` in the command working
directory. That directory is conventionally controlled by Cargo in a Rust
workspace. Cargo can remove it independently of Veln, and the working
directory can differ from the selected Veln package root.

Reusable classfiles are cache data, not durable command output. Their deletion
must affect performance only. A Veln-owned user cache avoids coupling cache
lifetime to another toolchain and avoids reserving a common package directory
name for Veln output.

## Terms

- The **user cache base** is the standard per-user cache directory selected by
  the host operating system.
- The **Veln cache root** is the directory below which Veln stores reusable
  toolchain cache data.
- An **override** is a non-empty absolute path supplied through
  `VELN_CACHE_DIR`.

## Proposed Behavior

When `VELN_CACHE_DIR` is not set, the toolchain selects the user cache base
from native environment values according to this decision table. Every value
is read as a native operating-system string. A value is usable only when it is
non-empty and lexically absolute.

| Host | User cache base |
| --- | --- |
| Unix other than macOS | Use `XDG_CACHE_HOME` when usable. Otherwise use the `.cache` child of a usable `HOME`. The base is unavailable when neither value is usable. |
| macOS | Use the `Library/Caches` child of a usable `HOME`. The base is unavailable otherwise. |
| Windows | Use a usable `LOCALAPPDATA`. The base is unavailable otherwise. |
| Other hosts | The base is unavailable. A valid override can still select the cache root. |

An unset, empty, or relative `XDG_CACHE_HOME` selects the `HOME` fallback; it
is not itself a configuration error. Other unusable host values make the base
unavailable when the table provides no fallback. Non-Unicode values remain
valid when the host path type can represent them and they satisfy the same
absolute-path rule. The toolchain must not perform a lossy Unicode conversion.

The default Veln cache root is the `veln` child of the selected user cache
base.

When `VELN_CACHE_DIR` is set to a non-empty absolute path, its value is the
Veln cache root. The toolchain must not append another `veln` component to the
override.

When `VELN_CACHE_DIR` is empty or is not an absolute path, a command that needs
the cache must fail with a command-level configuration error before it writes
cache data. A command that does not need the cache need not inspect the
variable.

If the host cannot provide a user cache base and no valid override is set, a
command that needs the cache must fail with a command-level configuration
error. It must not fall back to the package root, working directory, or
`target`.

The toolchain reads `VELN_CACHE_DIR` as a native operating-system string.
A non-Unicode override is valid when it is non-empty and lexically absolute.
Override validation does not canonicalize the path, remove `.` or `..`
components, or resolve filesystem identity. Existing symbolic-link components
and a cache root that is a symbolic link to a directory are accepted. Missing
components are created when the cache is first used.

A dangling symbolic link, a link to a non-directory object, or another path
failure is reported when the command first tries to use the cache root. The
command must not retry at another cache location. Two lexical paths that reach
the same filesystem object are both valid settings; the toolchain does not
promise to recognize them as the same configuration.

### Validation Point

The toolchain validates cache configuration only after an invocation has
successfully analyzed its sources, selected at least one executable entry or
test, produced the corresponding JVM program, and established that the Java
launcher is available. It validates once, immediately before the first cache
lookup or creation. A cache hit still requires validation because the selected
root identifies the entry.

Help, version output, and commands that do not execute a JVM program do not
inspect cache configuration. A source-analysis failure, a missing or blocked
`run` entry, a `test` invocation with no executable tests, or a missing Java
launcher also does not inspect cache configuration and does not create cache
data. The earlier applicable result is reported instead.

`test` validates the selected cache root before it starts any executable test
body. All tests in the invocation use that root. Invalid configuration must
not allow an earlier test to run before the invocation fails.

### Cache Root Availability

A cache-required invocation requires the selected cache root to support all
directory, coordination, validation, preparation, and publication operations
needed by that invocation. A missing root is created when possible. A regular
file or other non-directory object cannot be a cache root. A root that cannot
be created or used for a required operation produces a focused command-level
cache-root error before user code or a test body starts.

The toolchain determines usability from the required filesystem operations,
not from permission bits alone. It does not use a non-persistent fallback or
write substitute cache data below the system temporary directory, package
root, working directory, `target`, or another user cache location. A failed
operation may leave empty directories or unpublished preparation remnants
inside the selected root, but it must not leave a partial entry that a later
invocation accepts as published.

All persistent reusable JVM class cache entries used by `run` and `test` must
be below the Veln cache root. Neither command may select a cache location from
the command working directory or package root when no override is set.

Cache data is disposable. Missing or previously removed entries must produce a
cache miss and regeneration, not a semantic or execution failure. Cache
corruption must not cause an invalid entry to execute.

An entry that fails integrity validation is a cache miss. The invocation must
attempt to discard the invalid published entry and regenerate it. Corruption
alone must not fail the command when repair succeeds. The regenerated entry
must pass complete validation before publication and execution.

If the invalid entry cannot be discarded, or regeneration, validation, or
publication fails, the command reports a focused cache error and does not
start the JVM. An invalid entry that remains at the published path must be
revalidated and rejected by every later invocation. If removal succeeded but
regeneration failed, a later invocation observes a miss and retries generation.
Unpublished remnants are never cache hits and may be cleaned up by a later
invocation.

### Concurrent Preparation

Publication of an entry is externally atomic. An invocation can observe the
published key as absent, complete and valid, or invalid and unusable. It must
never accept or execute a partial entry. Each invocation validates the complete
entry that it will use before JVM startup.

Concurrent invocations may duplicate private preparation work. When one
invocation publishes a valid entry, another invocation preparing the same key
must revalidate and may reuse that winner. A valid published entry is immutable
for its key. A writer whose preparation, validation, publication, or cleanup
fails must not delete, replace, or invalidate a valid entry published by
another invocation.

If no invocation can obtain a valid winner, each affected command fails with a
focused cache error and does not start the JVM. Private remnants remain outside
the published lookup path. If a coordinating process stops, another invocation
must recover or reach a bounded cache-coordination error; it must not wait
forever. The coordination representation, waiting strategy, and bound are not
part of the external contract.

The Veln cache root does not establish a source-discovery exclusion. Source
ownership remains determined by package discovery rules even when an override
places the cache root below a package.

## Acceptance Model

The following table is the planned behavioral contract. The evidence column
names evidence to add during implementation; it does not describe tests that
already pass.

| Case | Environment or action | Required result | Planned primary evidence |
| --- | --- | --- | --- |
| Default user cache | `VELN_CACHE_DIR` is unset and `run` needs a JVM class cache entry | The entry is written below the `veln` child of the host user cache base and no cache entry is written below the package or working directory | CLI harness case with an isolated user cache base |
| Explicit override | `VELN_CACHE_DIR` names an absolute writable directory | The complete supplied path is used as the cache root | CLI harness case with an isolated override directory |
| Empty override | `VELN_CACHE_DIR` is set to an empty value and `run` needs the cache | The command reports a configuration error before writing cache data | CLI harness failure case |
| Relative override | `VELN_CACHE_DIR` names a relative path and `test` needs the cache | The command reports a configuration error before writing cache data | CLI harness failure case |
| User cache unavailable | No valid override is set and the host user cache base cannot be resolved | The command fails without writing below the package, working directory, or `target` | Cache-root resolution unit test |
| Deleted entry | A successful invocation populates an entry and that entry is removed before the same invocation is repeated | The later invocation regenerates the entry and preserves command output and status | JVM cache integration test |
| Working-directory independence | Equivalent package analysis is invoked from different directories without an override | Both invocations select the same host user cache root | CLI harness case |
| Source-discovery independence | An override points below a package root | Setting the override does not add an ignored source path or change the package source checksum | `veln-project` and CLI integration case |
| Unix XDG fallback | `XDG_CACHE_HOME` is unset, empty, or relative and `HOME` is absolute | The default root is the `veln` child of `HOME/.cache` | Table-driven cache-root resolver unit test |
| Unix cache base unavailable | Neither `XDG_CACHE_HOME` nor `HOME` supplies a usable Unix base | A cache-required command reports a configuration error without local fallback | Cache-root resolver unit test and CLI harness failure case |
| macOS cache base | `HOME` is an absolute native path on macOS | The default root is the `veln` child of `HOME/Library/Caches` | Table-driven cache-root resolver unit test and platform-conditional integration case |
| Windows cache base | `LOCALAPPDATA` is an absolute native path on Windows | The default root is its `veln` child | Table-driven cache-root resolver unit test and platform-conditional integration case |
| Non-Unicode host base | A supported host supplies an absolute non-Unicode cache-base value | The native path is accepted without lossy conversion | Platform-conditional cache-root resolver unit test |
| Non-Unicode override | `VELN_CACHE_DIR` contains a non-Unicode absolute native path | The complete native path is used as the cache root | Platform-conditional cache-root resolver and integration test |
| Lexical override | An absolute override contains `.` or `..`, does not exist yet, or contains a directory symbolic link | The value is accepted without canonicalization and filesystem operations follow the supplied path | Platform-conditional cache-root integration test |
| Invalid override has precedence | An empty or relative override is set while a valid host base is available | The command reports a configuration error and does not fall back to the host base | Cache-root resolver unit test and CLI harness failure case |
| Analysis failure precedence | Sources fail discovery, parsing, or semantic analysis while the override is invalid | The command reports the source failure without inspecting the override or writing cache data | CLI harness failure case |
| No executable tests | Test selection produces no executable test while the override is invalid | The command preserves the normal no-tests result and does not inspect the override | CLI harness case |
| Missing Java precedence | A runnable JVM program exists, Java is unavailable, and the override is invalid | The command reports the Java setup error without inspecting the override or creating cache data | CLI harness failure case |
| Non-executing invocation | Help, version output, or a command that does not execute a JVM program runs with an invalid override | The command preserves its normal result and does not create cache data | CLI harness cases |
| Multi-test validation atomicity | Multiple executable tests are selected while the override is invalid | Cache configuration fails before any test body starts | CLI harness failure case |
| Cache root is a file | The selected root is an existing regular file | The command reports a cache-root error, does not alter the file, and does not start the JVM | JVM cache integration test |
| Cache root creation failure | A required root or ancestor cannot be created | The command reports the filesystem cause without writing cache data elsewhere | Fault-injected unit test and platform-conditional CLI case |
| Cache root write failure | Coordination or publication below the selected root fails | The command reports a cache-root error, publishes no partial entry, and uses no fallback | Fault-injected JVM cache unit test |
| Corrupt entry repair | A published entry has altered class bytes, missing or extra files, or inconsistent metadata | The invalid entry is not executed; regeneration succeeds and preserves program output and status | JVM cache integration test |
| Corrupt entry removal failure | An invalid published entry cannot be removed | The command reports a cache error without JVM startup; a later invocation revalidates and retries repair | Fault-injected JVM cache unit test |
| Regeneration failure | Invalid-entry removal succeeds but preparation or validation fails | The command publishes no partial entry; a later invocation observes a miss and can regenerate | Fault-injected JVM cache unit test |
| Concurrent cold entry | Multiple invocations prepare the same missing key | Every successful invocation uses a complete validated winner and program results are unchanged | Barrier-controlled JVM cache concurrency test |
| Concurrent repair | Multiple invocations repair the same invalid key | They converge on a complete validated entry and no invocation executes the corrupt entry | Barrier-controlled JVM cache concurrency test |
| Failed writer isolation | One writer fails after another publishes a valid entry | The valid winner remains byte-for-byte valid and available to other invocations | Fault-injected JVM cache concurrency test |
| Abandoned coordination | A process stops while coordinating or preparing an entry | A later invocation recovers or reports a bounded cache-coordination error without executing remnants | Process-level JVM cache integration test |

## Compatibility Consequences

Existing reusable JVM class cache entries below `target` will no longer be
read. The first affected `run` or `test` invocation will populate the user
cache again.

Commands will no longer create the Veln JVM class cache below their working
directory. Scripts that inspect the current internal cache location must stop
depending on it.

An invalid `VELN_CACHE_DIR` will become an explicit configuration failure for
commands that require persistent cache data.

## Non-Goals

- Do not add a `cache` or `clean` subcommand.
- Do not specify the internal cache directory layout, entry names, locking
  representation, eviction policy, or size policy.
- Do not specify a durable output location for a future build command.
- Do not specify dependency download or package materialization caches.
- Do not add a manifest field for cache placement.
- Do not make cache presence part of language semantics or command output.
- Do not define source-discovery exclusions from cache placement.

## Verification

Implementation must add the evidence named in the acceptance model. Planned
local verification uses repository-relative commands:

```sh
bash scripts/agent-test -p veln-project
bash scripts/agent-test -p veln-cli --test toolchain_harness
```

## Completion Boundary

This proposal is complete when `run` and `test` use the specified cache-root
selection contract, the acceptance evidence passes, and the current behavior
is promoted to the matching command specification.

After completion, move this document to
`../reference/implemented-proposals/` and remove it from the proposal catalog.
