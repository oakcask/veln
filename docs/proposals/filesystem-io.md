---
role: proposal
update-when: The proposed filesystem integration, native path conversion, shared byte IO capabilities, provider ownership, seek behavior, fake filesystem contracts, or filesystem IO feasibility dependency changes.
---

# Filesystem And Shared Byte IO

Provide practical filesystem access with streaming binary IO, random access,
explicit filesystem selection, and scoped substitution through Veln effect
handlers. The same byte-processing helpers should work with files, network
streams, and memory.

The [design decisions](#design-decisions) resolve the initial scope. The
implemented static and bounded runtime prerequisites belong to the current
[filesystem and byte IO specification](../specification/standard-library-filesystem-io.md).
The complete-transfer helper outcome matrices are implemented. The remaining
F4 direct, host-translation, seek, namespace, and mutation behavior, F5 cleanup
behavior, and host integration evidence are planned.
Implementation selection remains blocked on the separate
[Filesystem IO Feasibility](filesystem-io-feasibility.md) proposal.

## Current Boundary And Scope

The production [filesystem boundary](../specification/effects.md#file-system-calls)
provides host-owned `Path` values and four compiler-known operations for text
reading, text writing, existence checks, and directory listing. The current
[filesystem and byte IO prototype](../specification/standard-library-filesystem-io.md)
owns the implemented source surface and bounded runtime foundation, including
complete-transfer helpers. It provides
no host filesystem execution. This proposal replaces the minimal production
boundary; retaining its source API is not required.

The current [network API](../specification/standard-library-networking.md)
provides a source-defined `net::IO` effect, a system handler, byte read outcomes,
and write outcomes that retain partial progress. The transport effect in
`crates/veln-stdlib/veln/transport.veln` is tied to
`net::NetError` and selects one stream through its handler context. It is
supporting evidence for effect adapters, not an existing general IO API.
The [execution boundary](../specification/execution.md#cleanup-limits) already
supports lexical cleanup, but does not define automatic close or flush.

| Included capability | Required outcome |
| --- | --- |
| Portable-path integration and explicit filesystem selection | Filesystem APIs use the current [portable path](../specification/standard-library-paths.md) value to name entries under explicit host and fake providers. |
| Read-only filesystem access | Open, metadata, directory enumeration, and byte reads work without requiring mutation capability. |
| Filesystem updates | Explicit open modes, byte writes, directory creation, removal, rename, and truncation have defined success and failure behavior. |
| Shared byte IO | Read, write, complete-transfer helpers, seek, and positional IO compose with explicit resources. |
| Resource lifecycle | Explicit close and scope exit expose defined release outcomes, retry behavior, cleanup order, and failure precedence. |
| Deterministic testing | A Veln fake implements stateful filesystem behavior and injected failures without opening host files. |

Memory mapping, file watching, advisory locks, hard links, detailed ACLs,
distributed filesystem consistency, crash transactions, buffering, confinement,
link creation, arbitrary native-name round trips, and asynchronous IO scheduling
are outside the initial scope. Unsupported capabilities fail explicitly.

## Paths And Filesystem Selection

The current [portable path specification](../specification/standard-library-paths.md) owns the pure
`path::Path` syntax, source API, and lexical operations. This proposal uses
that type for names within an explicit filesystem. A portable path does not
identify a filesystem, authorize access, or assert that an entry exists.

Filesystem operations receive an explicit filesystem resource and a portable
path, or an open resource that already retains its filesystem identity. Two
filesystem instances can be used in one computation, including copying between
them. Portable operations do not consult ambient process working-directory
state. Selecting a host root uses a host path at an explicit host boundary.

Native conversion is fallible. Unsupported names produce a typed error and do
not select a differently spelled entry. Display text is not a promised
round-trip representation of a native name. Canonicalization, link resolution,
and filesystem identity queries are effectful and separate from lexical path
operations.

Go's [filepath.Localize](https://pkg.go.dev/path/filepath#Localize)
demonstrates why portable validity and native representability are different
checks. Veln provides the implemented validated portable path instead of
accepting unchecked strings at every call.
Rust's [native Path conversion](https://doc.rust-lang.org/std/path/struct.Path.html#method.to_str)
shows why an arbitrary native name need not have a lossless Unicode spelling;
the initial API rejects native names without an exact supported Unicode
representation rather than promising arbitrary native-name round trips.

### Root Views And Confinement

A subtree view changes the namespace root. It does not by itself promise
race-resistant confinement against symbolic links or concurrent directory
replacement. A confinement capability, if provided, has a separate documented
guarantee and an explicit unsupported outcome on a backend that cannot enforce
it. Implementations must not advertise confinement based only on lexical
normalization, a string-prefix check, or a separate check before open.

Go explicitly distinguishes [Sub](https://pkg.go.dev/io/fs#Sub) from
[os.Root](https://pkg.go.dev/os#Root). Java's
[SecureDirectoryStream](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/nio/file/SecureDirectoryStream.html)
provides operations relative to an open directory only where the host provider
supports it. These sources justify separating namespace views from confinement;
they do not establish that every Veln backend can provide a sandbox.

## IO Capabilities And Effects

Separate filesystem namespace operations from byte IO. Read, write, seek,
positional read, positional write, and close are distinct capabilities.
Reading a configuration file must not require an implementation to provide
mutation operations. A network stream need not provide seek. Memory resources
can implement byte capabilities without filesystem access.

Operations carry explicit resource operands. Distinct nominal view types
enforce static capabilities. The implemented provider identity and dispatch
contract belongs to the
[current specification](../specification/standard-library-filesystem-io.md#provider-bound-dispatch).

The public substitutable boundaries are nominal effects implemented by ordinary
Veln handlers. A system handler translates those operations into real host IO;
a memory or fake handler implements them without installing a system handler.
Generic algorithms retain their declared byte capabilities until an adapter
handles them. The system adapter exposes the appropriate host effects; a fake
does not add a host filesystem effect merely to manufacture handles. There is
no implicit host fallback for an unhandled nominal operation or an unknown
resource. This follows the existing nominal-effect boundary rather than adding
an implicit system provider to every helper.

The byte layer uses `ByteChunk`, `ByteCount`, and `ByteOffset`, with a common
typed IO error vocabulary that is independent of `net::NetError`. Filesystem
and network adapters retain domain context when translating failures. The
[implemented source surface and transfer behavior](../specification/standard-library-filesystem-io.md)
fix the current type boundaries and complete-transfer outcomes. The remaining
rows below define planned runtime semantics for the direct facade, positional
operations, and host adapters.

| Operation family | Observable contract |
| --- | --- |
| Bounded sequential read | Returns at most the requested byte count, advances by the bytes delivered, and distinguishes data, clean end, and failure. A short read alone is not end-of-input. |
| Sequential write | Reports the committed prefix of the input and advances according to committed bytes. Failure does not imply rollback. |
| Positional read/write | Uses an explicit absolute byte offset without changing the sequential cursor. Lack of support is explicit, not an unsynchronized seek-and-transfer emulation. |

The current specification defines complete-transfer handling for positive,
zero-size, failed, and malformed provider outcomes. The remaining common
outcome design must preserve data or progress when a direct or host operation
also fails. Unknown host progress is represented explicitly, never fabricated
as zero or as a known exact count. The outcome table below fixes the remaining
direct and host behavior.

Go's [Reader and Writer contracts](https://pkg.go.dev/io) motivate capability
composition and preserving progress alongside failure. Veln adapts these to
immutable chunks and typed outcomes instead of copying mutable buffer and
`(n, error)` conventions. Go's [ReaderAt](https://pkg.go.dev/io#ReaderAt)
motivates the separate positional capability; its concurrent-call contract
does not make a sequence of Veln seek and read operations atomic.

## Seek And File State

Seek uses a typed origin: start with a non-negative absolute offset, current
with a signed displacement, or end with a signed displacement. It returns the
resulting absolute offset. Offsets count bytes, not characters.

| State and operation | Proposed result and state |
| --- | --- |
| Open seekable resource; valid target within the file | Success; cursor becomes the target. Contents and size are unchanged. |
| Open seekable resource; valid target beyond end | Success; cursor becomes the target. Seeking alone does not extend the file. A positive-size read there returns clean end. |
| Target is negative or arithmetic overflows | Typed invalid-input failure; cursor, contents, and size are unchanged. |
| Resource has no seek capability | Cannot be passed as `io::Seeker`; acquiring a dynamic seek view returns `Unsupported` without changing state. |
| Closed resource | Typed closed-resource failure; no transfer or cursor change. |
| Positional read/write | Sequential cursor is unchanged, including after an operation that reports partial progress. |

Each independent open has its own cursor and close state. Copying a handle
refers to the same open resource and shares those states. Two opens of one
file refer to the same underlying contents, subject to the provider's stated
external-consistency limits. They do not share cursors merely because their
paths are equal.

Rust's [Seek](https://doc.rust-lang.org/std/io/trait.Seek.html) motivates a
typed origin rather than integer `whence`. This proposal additionally fixes
invalid-target state preservation and seeking beyond end. The mutation table
below fixes gap bytes, append, and failed-seek state. Buffering is deferred.

## Filesystem Updates And Resource Lifetime

Opening distinguishes access rights, creation, exclusive creation, truncation,
and append behavior explicitly. Invalid combinations fail before modifying the
filesystem. Read-only open does not create or truncate. Exclusive creation
does not use an existence check followed by a separate create; when the entry
already exists, its contents remain unchanged.

Metadata identifies at least file kind and byte length where meaningful.
Directory entries carry their names and kind information rather than only
repeating complete paths. Enumeration returns one sorted list, with the
concurrent-change limits specified below; it does not expose pagination.
Missing metadata and unsupported operations must not be encoded as invented
zero values or successful no-ops.

The mutation table defines creation, removal, rename, truncation, append, and
sync, including failed-operation state. A successful byte
write or close does not itself promise crash durability. Buffered flush and
storage synchronization are distinct concepts.

The implemented owner lifecycle belongs to the
[current specification](../specification/standard-library-filesystem-io.md#owner-closure-and-restricted-release).
F5 defines explicit-close results, retry, reverse cleanup, and primary or
related failure projection within the existing
[execution cleanup limits](../specification/execution.md#cleanup-limits).
Independent fake executions must remain isolated. Existing `defer` cannot
discard a meaningful close failure and is not a substitute for specifying
these outcomes.

Java's [FileChannel](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/nio/channels/FileChannel.html)
supports exclusive creation and positional IO, but leaves append atomicity
and gap contents dependent on the host contract. Its
[seekable channel boundary](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/nio/channels/SeekableByteChannel.html)
also permits interruption to close a channel. The initial contract preserves
these backend limits through explicit outcomes;
a portable fake does not establish stronger host guarantees.

## Fake Filesystem And Conformance

Deterministic tests inject scoped fake handlers written in separate Veln test
support. The production system handler uses real host effects by default;
environment variables and system properties do not select fake behavior.
The fake can create resources without opening host files.

The fake exposes behavior equivalent to distinct directory bindings, file
contents, and per-open state. These are observable distinctions, not a required
internal data structure. In particular, two opens share updates but have
independent offsets, and aliases of one open share close and cursor state.
State mutation shared by child tasks is synchronized. Test executions do not
share implicit global filesystem state.

Failure plans can select an operation and occurrence, return a typed failure,
and describe transferred bytes or committed changes. An operation log records
the target resource, request, outcome, and observable progress. Tests can
assert the resulting bytes, cursor, namespace changes, and released resources.
The fake does not claim to simulate every host's permissions, scheduling,
crash durability, or link semantics.

Go's [testing/fstest](https://pkg.go.dev/testing/fstest) supplies both small
memory trees and a reusable implementation checker. Adopt that shared
conformance-suite approach. Its `MapFS` supports read-only access and does not
make concurrent map mutation safe; Veln additionally needs mutable contents,
per-open lifecycle, synchronized state, and scripted failures.

## Acceptance Model

The checked feasibility foundation belongs to the
[current specification](../specification/standard-library-filesystem-io.md).
The evidence below is planned for the remaining behavior. The implemented
complete-transfer matrices and their checked evidence belong to the
[current specification](../specification/standard-library-filesystem-io.md#references).
Expected byte sequences and state transitions come from the contracts and
decision tables in this proposal, not from copying the implementation's
output. Fake tests do not establish real host behavior.

| Concern | Observable acceptance | Planned evidence |
| --- | --- | --- |
| Host conversion | An unrepresentable name fails without selecting another entry; native and portable types cannot be interchanged implicitly. | Companion tests plus focused host-conversion and compiler type-boundary tests. |
| Remaining partial transfer | Direct operations and complete helpers preserve bytes and progress; partial writes retry only an uncommitted suffix; unknown progress stops retry. | Scripted Veln fake tests with independent expected contents and operation logs. |
| Seek and positional IO | Origin arithmetic, invalid targets, beyond-end reads, independent opens, aliases, and positional cursor preservation obey the state table. | Companion tests shared by memory and supported system providers; host tests for native translation. |
| Namespace updates | Open modes, exclusive create, directory operations, rename, and truncation obey the mutation transition table, including failure preservation. | Shared provider conformance tests and focused real-host integration cases. |
| Errors and cleanup | Stable error kinds retain operation and path context; close and cleanup transitions preserve progress and the selected primary failure. | Companion tests plus human/JSON execution coverage where runtime cleanup diagnostics apply. |
| Real host default | A system handler reaches real files without fake selectors, while an injected fake requires no host-file setup. | Focused host integration and effect-boundary policy checks. |

Standard-library API behavior belongs in `crates/veln-stdlib/veln/` companion
tests. Compiler, backend, and host integration cases must identify their own
contract under the [test placement policy](../reference/toolchain-test-harness.md).
No broad stress or performance claim is part of this proposal.

## Design Decisions

These decisions replace the former open questions. The adapter and capability
choices are Veln design inferences, not guarantees supplied by the comparative
sources. Implemented prerequisites belong to the
[current specification](../specification/standard-library-filesystem-io.md).
The table lists remaining runtime decisions; it is not evidence that their
runtime support exists.

| Concern | Selected initial design | Acceptance obligation |
| --- | --- | --- |
| Failure-plan state | Extend the provider-bound Veln fake state with scripted failures, resource cursors, close facts, and operation progress as ordinary Veln data. | A scripted failure changes only the selected provider instance and records the commit facts required by the operation outcome matrix. |
| Native paths | Keep the current host-owned `Path` as the explicit native boundary; `path::Path` remains portable. Support only exactly representable names initially. | Nonrepresentable names fail with `UnrepresentablePath`; display is never used as a conversion. |
| Links and confinement | Follow links for ordinary open; explicit follow/no-follow metadata. No confinement or link-creation capability initially. A subtree is only a namespace view. | Dangling links, cycles, unsupported no-follow, and a link outside a subtree cannot be misreported as confined access. |
| Remaining IO outcomes | Preserve delivered bytes, exact or unknown progress, and typed errors for the direct facade and host adapters. The current specification owns complete-transfer helper outcomes. | Scripted partial-failure, empty-request, no-progress, and unknown-progress cases below, excluding the implemented helper matrices. |
| Seek and mutation | Absolute logical cursor, host-defined gap bytes, append-only writer views, no-replacement atomic rename where supported, and explicit mutation commit facts. | Shared transition cases plus host tests for unsupported and unknown host outcomes. |
| Concurrency and cancellation | Nonblocking per-open admission returns `Busy` for overlapping transfers, seek, truncate, or sync. Close can invalidate an in-flight operation. No bounded host cancellation promise. | Gated overlap, close/interruption races, and reconciliation after unknown cursor or close state. |
| Cleanup and durability | Reverse-publication cleanup, continue after failures, network-compatible close commit facts and abrupt cleanup precedence. No automatic sync. | Known-false, known-true, and unknown close failures, retries, and body-failure precedence. |
| Transport and buffering | Add explicit network-to-byte adapters; keep the existing domain-specific transport facade. Buffering is excluded from this implementation. | File/network/memory copy and preserved network context; no file helper acquires `net::IO`. |

### Source Surface And Static Capabilities

The compiler-checked declarations, opaque identities, file views, effects,
factories, projections, and slot accessors are implemented and specified by
[Standard-library filesystem and byte IO](../specification/standard-library-filesystem-io.md).
This proposal no longer owns that static contract.

Runtime implementation must make every field returned by one open refer to the
same resource. It must not grant a capability absent from the requested file
view or upgrade a read-only filesystem. If a provider cannot supply an entire
requested view, it must return `Unsupported`, release any partial host open,
and avoid publishing a partial view. Network adapters must publish only their
declared reader, writer, and closer views.

Runtime mode validation must reject create-new with truncation before host
access. Append must remain a separate entry point and must not grant read,
truncate, seek, or positional-write views. Optional capability discovery is
outside the initial API.

### Provider Lifecycle

The current
[owner closure and restricted release specification](../specification/standard-library-filesystem-io.md#owner-closure-and-restricted-release)
owns the implemented F3 lifecycle boundary. F5 reuses that boundary for the
remaining explicit-close outcomes, retries, reverse-publication cleanup, and
ordered primary and related cleanup-failure projection described in
[Concurrent Operations And Cleanup](#concurrent-operations-and-cleanup).

### Outcomes And Error Translation

The ADTs below describe payloads, not a new syntax. `Progress` is `Known(count:
ByteCount)` or `Unknown(known_prefix: ByteCount)`. Unknown means at least the
reported prefix is known, but more may have transferred; it never permits an
automatic retry. `Commit` is `No`, `Yes`, or `Unknown`.

| Type | Variants and meaning |
| --- | --- |
| `io::ReadOutcome` | `Data(bytes: ByteChunk)`, `End`, `ReadFailed(bytes: ByteChunk, consumed: Progress, error: io::Error)`. Failure bytes are the exact delivered prefix; consumed input may exceed it only when reported explicitly. |
| `io::WriteOutcome` | `Written(count: ByteCount)`, `WriteFailed(committed: Progress, error: io::Error)`. A known count cannot exceed the request. |
| `io::ExactOutcome` | `Complete(bytes: ByteChunk)`, `Short(bytes: ByteChunk)`, `ExactFailed(bytes: ByteChunk, consumed: Progress, error: io::Error)`. `Short` means clean end before the requested size. |
| `fs::MutationOutcome` | `Applied`, `MutationFailed(committed: Commit, error: io::Error)`. Unknown does not promise rollback or invite an automatic retry. |
| `io::CloseOutcome` | `Closed`, `CloseFailed(released: Commit, error: io::Error)`. Release facts describe ownership, not byte durability. |
| `io::Error` | Record: `operation: String`, `kind: io::ErrorKind`, `path: Option<path::Path>`, `other_path: Option<path::Path>`, `network: Option<net::NetError>`, `commit: Option<Commit>`, `message: String`. Commit carries namespace changes from failed open; transfer progress remains in its outcome ADT. Message is explanatory and not a stable comparison key. |

`ErrorKind` contains `InvalidInput`, `NotFound`, `AlreadyExists`,
`PermissionDenied`, `NotDirectory`, `IsDirectory`, `DirectoryNotEmpty`,
`LinkLoop`, `UnrepresentablePath`, `CrossFilesystem`, `Unsupported`, `Closed`,
`Busy`, `InvalidResource`, `Cancelled`, `NoProgress`, and `Other`.
Translate only facts established by a typed host exception or return value;
do not parse localized exception text into a category. An unclassified host
failure is `Other`. Network adapters retain the original `NetError` as context.
Filesystem errors retain both paths for rename. Unknown byte progress is
independent of error category and namespace/ownership commit facts.

| Input or outcome | Required state and helper behavior |
| --- | --- |
| Zero-size direct read or write on a live permitted view | Return empty `Data` or `Written(0)` without a provider transfer. Validate owner, capability, and closed state first; zero size cannot revive a closed resource. |
| Positive request, empty successful read or zero successful write | Direct facade reports `NoProgress`. No invented end or success. |
| Short data read | Deliver the returned bytes. It is not end. |
| Read failure with delivered bytes | Include those bytes and consumption in the failure. |
| Partial successful direct write | Report the committed prefix without implying completion of the submitted chunk. |
| Exception after a host transfer attempt | Preserve independently established prefix/progress; otherwise report unknown progress. Sequential cursor becomes unavailable until successful absolute `Start` seek or close if its value is unknown. Relative seek fails with `InvalidResource` in that state. |
| Positional failure | Preserve the sequential cursor, including when transfer progress is unknown. Unknown content changes remain observable; no seek-and-transfer emulation. |
| Direct-facade count outside the submitted bounds, or counter overflow | Stop with `InvalidInput` for caller input or `InvalidResource` for an invalid provider reply. Do not wrap counts. |

### Namespace, Seek, And Mutation Transitions

Paths are interpreted under the explicit filesystem root. The implemented
portable path type already rejects absolute paths and parent components;
filesystem operations retain that validated boundary. This lexical rule is not a
confinement guarantee: followed links and concurrent directory replacement may escape a subtree view.

`Metadata` contains `kind: fs::Kind` and `length: Option<ByteCount>`.
`Kind` is `RegularFile`, `Directory`, `SymbolicLink`, or `Other`; length is
present for regular files and absent when not meaningful. `Entry` contains
`name: path::Path` (one relative component) and `kind: fs::Kind` obtained
without following the entry's final link. Enumeration returns the observed
entries sorted by Unicode scalar name order, without duplicates. It is not an
atomic snapshot under external mutation; a conversion or enumeration error
returns no partial list. There is no fabricated zero size or missing entry.

| Operation and state | Success | Failure or limit |
| --- | --- | --- |
| Open existing; read-only | New independent cursor at zero; no creation or truncation. | Missing entry: `NotFound`; directory: `IsDirectory`; no mutation. |
| Writable open with create/truncate | Apply the declared mode; a successful truncating open has size zero. | Validate mode before host access. If creation/truncation may have committed before a later open failure, retain that fact in error context; do not claim rollback. |
| Exclusive `CreateNew` | Atomically create a new regular file. | Existing entry: `AlreadyExists`, unchanged. Unsupported atomic create: `Unsupported` before mutation. |
| Follow/no-follow | Ordinary open follows links. `metadata(..., NoFollow)` describes the final link; `Follow` describes its target. | Dangling target: `NotFound`; cycle: `LinkLoop` when identified. A backend unable to provide no-follow returns `Unsupported`. Intermediate components still follow host traversal rules. |
| Seek beyond end, then write | Seek alone preserves size. A subsequent successful write extends size. | Gap byte values are provider-defined and not a portable zero-fill promise. Fake policy fills zeros, but conformance never infers host zeros from that policy. |
| Failed host seek | Preserve cursor when the host proves no change. | Otherwise mark sequential cursor unknown. A successful absolute `Start` seek restores it; `Current` and `End` are rejected until then. No transfer is implied. |
| Append write | Each admitted write targets the then-current end. | No seek/positional-write view. No record-atomicity guarantee against another open or process; committed prefix is still reported. |
| Truncate to smaller size | Size becomes the target; that open's cursor becomes `min(old_cursor, target)`. Other opens keep their cursors. | A target at or above current size is a no-op. Negative/overflow input fails before mutation. Unknown host mutation marks cursor unknown and reports unknown commit. |
| Create directory | Create exactly one missing directory; do not create parents. | Existing entry: `AlreadyExists`; missing parent: `NotFound`. No recursive operation. |
| Remove file/directory | File removal unlinks the final entry without following its final link; directory removal requires an empty directory. | Wrong kind is rejected. Nonempty directory: `DirectoryNotEmpty`. Existing open handles follow provider host limits; they must never switch to a replacement entry. |
| Rename | Move one binding within one filesystem, without replacing an existing destination, as one atomic namespace operation. | Existing destination: `AlreadyExists`; cross-filesystem: `CrossFilesystem`; no atomic no-replace primitive: `Unsupported` before mutation. Never emulate using check-then-move, copy, or delete. Unknown commit is reported if host failure cannot establish the result. |
| Sync | Request host file data and file-metadata synchronization. | Unsupported host/provider: `Unsupported`. Success does not promise parent-directory persistence, remote-server crash survival, or a multi-file transaction. Close/write success does not imply sync. |

Host `Path` remains distinct from portable `path::Path`. `process::cwd()`
continues to return that host type; it does not become a portable filesystem
path. `fs::system(root)` is the explicit bridge and captures the root at
creation. Replace pure `fs::path` with `fs::native_path(text: String) ->
Result<Path, io::Error>` and add `fs::display_native(path: Path) -> String`;
both are pure. Conversion validates native syntax without host IO. Display is
for humans, not a lossless parser input. `fs::system` resolves a relative host
root once against the current process directory, checks it is a directory, and
retains that selection; it never re-resolves against a later working directory.
No arbitrary bytes or UTF-16 code-unit API is introduced. Unrepresentable
directory names fail enumeration. Remove the four old compiler-known text,
existence, and listing functions when the replacement lands; migrate callers
to explicit filesystem selection and byte/text helpers. Keep the `fs` coarse
effect label for real host access. No source-compatibility aliases are required.

The [Java FileChannel contract](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/nio/channels/FileChannel.html)
informs the positional, append, and concurrency choices: Veln deliberately
chooses `Busy` instead of host blocking and supplies no append atomicity.
The [SeekableByteChannel contract](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/nio/channels/SeekableByteChannel.html)
informs the gap and interruption limits; Veln preserves unspecified host gap
contents instead of imposing a zero-fill layer. These APIs do not establish
Veln's provider identities or fake semantics.

### Concurrent Operations And Cleanup

All views and aliases of one open share admission and lifetime state. At most
one transfer, seek, truncate, or sync is admitted at a time; overlap returns
`Busy` rather than blocking while holding fake state. This also applies to
positional calls initially. Independent opens can run concurrently and have
no ordering guarantee for overlapping writes. No compound operation spanning
separate calls is implicitly atomic.

| State and event | Next state and observation |
| --- | --- |
| Open; explicit close wins admission | Stop admitting new operations; attempt release. An already admitted operation may complete first or fail with established progress if host close interrupts it. |
| Close returns success or failure with release `Yes` | Closed; all aliases unusable. Return the reported failure even if release completed. Repeated close returns `Closed` without another provider close. |
| Close fails with release `No` | Preserve the prior usable state and cleanup obligation; explicit retry is permitted. |
| Close fails with release `Unknown` | Unavailable for transfers, retain release obligation, permit explicit close retry. Never fall back to another provider. |
| Cancellation during an admitted host call | Preserve proven progress; interruption can close the entire open and all its views. Mark release/cursor facts unknown when unproved. No bounded interruption latency or rollback guarantee. |
| Owner cleanup within the implemented lifecycle boundary | Attempt all retained resources in reverse publication order, even after one close fails and when child tasks retain aliases. |
| Cleanup has unsuccessful or unknown release | Raise an abrupt cleanup failure after attempting all resources. Keep failed release obligations until final owner teardown; never report successful release or transfer them to a different invocation. External termination can still leak host resources under existing cleanup limits. |
| Successful body, ordinary `Err`, or `?` propagation plus cleanup failure | Cleanup failure is primary, matching network-handler cleanup. Prior ordinary error is retained as related context. |
| Runtime failure, contract failure, or cancellation plus cleanup failure | Existing abrupt failure remains primary. Append cleanup failures in reverse publication order as related context. |

No buffered helper or implicit flush is added. A scoped helper is expressed
with the owner region and existing cleanup; no callback result is silently
converted from a cleanup failure to `Ok` or an ordinary `Err`. A future helper
that catches close failures into typed results needs a separate contract.

Network-to-byte adapters bind reader/writer views to their original
`net::Stream` and translate `ReadOutcome`/`WriteOutcome` into the shared ADTs,
including its exact known progress and original `NetError`. The adapter
handler exposes `net::IO`; generic byte helpers expose only byte effects.
The existing `transport::DuplexStream` remains a network-specific facade rather
than forcing files to implement it. Buffer ownership, flush ordering, and seek
invalidation belong to a separate buffering proposal before such helpers exist.

### Feasibility Dependency

[Filesystem IO Feasibility](filesystem-io-feasibility.md) owns the remaining
F4–F5 prototype scope, evidence, and completion conditions. It is independently
selectable from Ready. The current provider foundation, together with the
implemented complete-transfer matrices and the planned remaining outcome,
transition, and complete cleanup tables here, remains the design contract
exercised by that work.
Implementing the complete production filesystem runtime is not its
prerequisite.

## Implementation Readiness And Promotion

The initial design decisions and acceptance tables are fixed above. Complete
the separate feasibility proposal before moving this proposal to Ready.
The adapter extension must be reviewed together with its source API,
not inferred from the existing network handles. A smaller independently complete
proposal may be split out only when it retains explicit ownership and outcome
contracts.

When implementation lands, update the smallest current specifications for
effects, shared IO, filesystem operations, and execution ownership alongside
the checked evidence. Add documentation routes only for implemented surfaces.
Remove implemented sections from this proposal, and remove the page and its
catalog entry when no unimplemented target remains.
