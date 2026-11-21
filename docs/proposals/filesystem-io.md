---
role: proposal
update-when: The proposed filesystem integration, native path conversion, shared byte IO capabilities, provider ownership, seek behavior, fake filesystem contracts, or acceptance gates change.
---

# Filesystem And Shared Byte IO

Provide practical filesystem access with streaming binary IO, random access,
explicit filesystem selection, and scoped substitution through Veln effect
handlers. The same byte-processing helpers should work with files, network
streams, and memory.

The [design decisions](#design-decisions) resolve the initial scope. The
acceptance model remains the full target. Source signatures below define the
proposed replacement API; prototype evidence does not establish that the full
API is implemented. Implementation selection remains blocked on the
provider-bound dispatch feasibility gate.

## Current Boundary And Scope

The current [filesystem boundary](../specification/effects.md#file-system-calls)
provides host-owned `Path` values and four compiler-known operations for text
reading, text writing, existence checks, and directory listing. It has no public
file handle, seek operation, or nominal filesystem handler. This proposal
replaces that minimal boundary; retaining its source API is not required.

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
| Resource lifecycle | Close and scope exit preserve provider identity and prevent accidental access to another provider. |
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

Operations carry explicit resource operands. The active handler supplies an
implementation, while a resource retains its creating provider and owner.
Nesting another handler must not reinterpret an existing resource as a resource
of the newly active provider. Distinct nominal view types enforce static
capabilities; the provider-bound dispatch prerequisite is specified below.

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
source surface and outcome types below fix these boundaries.

| Operation family | Observable contract |
| --- | --- |
| Bounded sequential read | Returns at most the requested byte count, advances by the bytes delivered, and distinguishes data, clean end, and failure. A short read alone is not end-of-input. |
| Sequential write | Reports the committed prefix of the input and advances according to committed bytes. Failure does not imply rollback. |
| Complete read | Distinguishes obtaining the requested amount from reaching end early, and preserves obtained bytes on failure. |
| Complete write | Repeats only the uncommitted suffix after partial success; stops on failure and preserves total progress. |
| Copy | Names independent input and output resources, preserves output progress, and terminates on end or failure. Failure can consume more input than it commits to output; the outcome must retain both facts. |
| Positional read/write | Uses an explicit absolute byte offset without changing the sequential cursor. Lack of support is explicit, not an unsynchronized seek-and-transfer emulation. |

For a positive-size request, successful empty reads or zero-progress writes
must not cause a helper to spin indefinitely or silently report completion.
The common outcome design must preserve data or progress when an operation
also fails. Unknown host progress is represented explicitly, never fabricated
as zero or as a known exact count. A helper must not automatically retry a
failure with unknown progress. The outcome table below fixes zero-size
behavior and the outcome ADTs.

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

An owner scope releases retained resources on normal return, propagated
errors, runtime failure, and task cancellation within the existing
[execution cleanup limits](../specification/execution.md#cleanup-limits).
Explicit close releases a resource earlier. Release facts describe the
underlying resource independently of whether a host wrapper still admits IO.
A wrapper marked closed after an error does not establish successful release;
keep release Unknown until it is proved. Closing or leaving its owner
scope makes all aliases unusable; later calls cannot fall back to host IO or
another provider. The owner stops publishing new resources when scope cleanup
starts. A child operation completing an open after that point releases the new
resource instead of publishing a usable handle.

Child tasks inherit the creating scope's handlers and resource ownership;
they do not extend a parent's owner lifetime merely by retaining a handle.
Independent executions construct independent fake state. Close and scope exit
use the cleanup transitions below. Existing `defer` cannot discard a meaningful close
failure and is not a substitute for specifying these outcomes.

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

The rows below define the full acceptance target. The partial prototype
evidence at the feasibility gate does not establish a complete row.
Expected byte sequences and state transitions
come from the contracts and decision tables in this proposal, not from
copying the implementation's output. Fake tests do not establish real host
behavior.

| Concern | Observable acceptance | Planned evidence |
| --- | --- | --- |
| Host conversion | An unrepresentable name fails without selecting another entry; native and portable types cannot be interchanged implicitly. | Companion tests plus focused host-conversion and compiler type-boundary tests. |
| Multiple resources | Copy between two resources and two filesystem instances reaches the intended input and output under nested handlers. | Companion tests with distinct bytes and provider traces. |
| Partial transfer | Short reads are data; complete helpers preserve bytes and progress; partial writes retry only an uncommitted suffix; unknown progress stops retry. | Scripted Veln fake tests with independent expected contents and operation logs. |
| Seek and positional IO | Origin arithmetic, invalid targets, beyond-end reads, independent opens, aliases, and positional cursor preservation obey the state table. | Companion tests shared by memory and supported system providers; host tests for native translation. |
| Namespace updates | Open modes, exclusive create, directory operations, rename, and truncation obey the mutation transition table, including failure preservation. | Shared provider conformance tests and focused real-host integration cases. |
| Ownership | Nested scopes do not redirect handles; scope exit closes aliases; child tasks cannot publish into a closing owner. Cleanup runs during error and cancellation unwinding. | Veln companion tests for provider behavior and JVM owner/handler integration tests. |
| Errors and cleanup | Stable error kinds retain operation and path context; close and cleanup transitions preserve progress and the selected primary failure. | Companion tests plus human/JSON execution coverage where runtime cleanup diagnostics apply. |
| Real host default | A system handler reaches real files without fake selectors, while an injected fake requires no host-file setup. | Focused host integration and effect-boundary policy checks. |

Standard-library API behavior belongs in `crates/veln-stdlib/veln/` companion
tests. Compiler, backend, and host integration cases must identify their own
contract under the [test placement policy](../reference/toolchain-test-harness.md).
No broad stress or performance claim is part of this proposal.

## Design Decisions

These decisions replace the former open questions. The adapter and capability
choices are Veln design inferences, not guarantees supplied by the comparative
sources. They are proposed contracts, not evidence that the API or runtime
support exists.

| Concern | Selected initial design | Acceptance obligation |
| --- | --- | --- |
| Public API | Nominal capability views and separate namespace and byte effects; no traits, implicit capability coercions, or new effect syntax. | Type-check the source surface below; reject a reader passed to write or a stream passed to seek. |
| Provider dispatch | Opaque runtime tokens retain their owner and creating operation handlers. Extend the runtime resource boundary explicitly; do not encode public handles as integers or host objects. | Two Veln providers, nested shadowing, wrong-owner publication, closed-owner use, and cross-provider copy at the feasibility gate. |
| Fake state | A synchronized `io_provider::State<T>` cell carries each fake filesystem's immutable state. Resource slots, file identities, cursors, close facts, and failure plans are ordinary Veln data. The runtime extension provides atomic storage, not fake behavior. | Concurrent children share updates; failure and cancellation cannot strand the state lock; independent fake instances remain independent. |
| Native paths | Keep the current host-owned `Path` as the explicit native boundary; `path::Path` remains portable. Support only exactly representable names initially. | Nonrepresentable names fail with `UnrepresentablePath`; display is never used as a conversion. |
| Links and confinement | Follow links for ordinary open; explicit follow/no-follow metadata. No confinement or link-creation capability initially. A subtree is only a namespace view. | Dangling links, cycles, unsupported no-follow, and a link outside a subtree cannot be misreported as confined access. |
| IO outcomes | Preserve delivered bytes, exact or unknown progress, typed errors, and copy's independent consumed/committed counts. | Scripted partial-failure, empty-request, no-progress, and unknown-progress cases below. |
| Seek and mutation | Absolute logical cursor, host-defined gap bytes, append-only writer views, no-replacement atomic rename where supported, and explicit mutation commit facts. | Shared transition cases plus host tests for unsupported and unknown host outcomes. |
| Concurrency and cancellation | Nonblocking per-open admission returns `Busy` for overlapping transfers, seek, truncate, or sync. Close can invalidate an in-flight operation. No bounded host cancellation promise. | Gated overlap, close/interruption races, and reconciliation after unknown cursor or close state. |
| Cleanup and durability | Reverse-publication cleanup, continue after failures, network-compatible close commit facts and abrupt cleanup precedence. No automatic sync. | Known-false, known-true, and unknown close failures, retries, and body-failure precedence. |
| Transport and buffering | Add explicit network-to-byte adapters; keep the existing domain-specific transport facade. Buffering is excluded from this implementation. | File/network/memory copy and preserved network context; no file helper acquires `net::IO`. |

### Source Surface And Static Capabilities

The following are signature contracts, not executable declaration syntax.
`Result`, records, ADTs, and nominal effects use existing Veln features.
Opaque provider and capability values require the runtime extension below.
No capability can be constructed by casting a number or a host object.

| Namespace | Source signatures and effects |
| --- | --- |
| Filesystem selection | `fs::system(root: Path) -> Result<fs::MutableFilesystem, io::Error> effects [fs::Host, io_provider::Scope]`; `fs::read_only(fs: fs::MutableFilesystem) -> fs::Filesystem` is pure. `fs::system_handler()` handles `fs::Host` and exposes coarse `fs` plus `concurrency` for owner registration; creating a fake filesystem does not require `fs::Host`. |
| Namespace read effect `fs::Read` | `open_read(fs: fs::Filesystem, path: path::Path) -> Result<fs::ReadFile, io::Error>`; `metadata(fs: fs::Filesystem, path: path::Path, follow: fs::FollowLinks) -> Result<fs::Metadata, io::Error>`; `entries(fs: fs::Filesystem, path: path::Path) -> Result<List<fs::Entry>, io::Error>`. Public facade functions have the same operands/results and effect `[fs::Read]`. |
| Namespace mutation effect `fs::Mutate` | `open_write(fs: fs::MutableFilesystem, path: path::Path, mode: fs::WriteMode) -> Result<fs::WriteFile, io::Error>`; `open_read_write(fs: fs::MutableFilesystem, path: path::Path, mode: fs::UpdateMode) -> Result<fs::UpdateFile, io::Error>`; `open_append(fs: fs::MutableFilesystem, path: path::Path, create: fs::CreateMode) -> Result<fs::AppendFile, io::Error>`; `create_directory(fs: fs::MutableFilesystem, path: path::Path)`, `remove_file(fs: fs::MutableFilesystem, path: path::Path)`, `remove_directory(fs: fs::MutableFilesystem, path: path::Path)`, and `rename(fs: fs::MutableFilesystem, from: path::Path, to: path::Path)` return `fs::MutationOutcome`. Facades expose `[fs::Mutate]`. |
| Byte read/write | `io::read(reader: io::Reader, maximum: ByteCount) -> io::ReadOutcome effects [io::Read]`; `io::write(writer: io::Writer, bytes: ByteChunk) -> io::WriteOutcome effects [io::Write]`. Each nominal effect declares the matching operation with the same operand/result types. |
| Cursor and positional IO | `io::seek(seeker: io::Seeker, origin: io::SeekOrigin) -> Result<ByteOffset, io::Error> effects [io::Seek]`; `io::read_at(reader: io::ReaderAt, offset: ByteOffset, maximum: ByteCount) -> io::ReadOutcome effects [io::ReadAt]`; `io::write_at(writer: io::WriterAt, offset: ByteOffset, bytes: ByteChunk) -> io::WriteOutcome effects [io::WriteAt]`. Each effect declares the matching operation. |
| Lifetime and storage | `io::close(closer: io::Closer) -> io::CloseOutcome effects [io::Close]`; `io::sync(syncer: io::Syncer) -> Result<(), io::Error> effects [io::Sync]`; `fs::truncate(file: fs::Truncater, size: ByteOffset) -> fs::MutationOutcome effects [fs::Mutate]`. Close and sync effects declare the matching operation. |
| Complete transfers | `io::read_exact(reader: io::Reader, count: ByteCount) -> io::ExactOutcome effects [io::Read]`; `io::write_all(writer: io::Writer, bytes: ByteChunk) -> io::WriteOutcome effects [io::Write]`; `io::copy(reader: io::Reader, writer: io::Writer, chunk_size: ByteCount) -> io::CopyOutcome effects [io::Read, io::Write]`. |

`fs::Host` declares `system(root: Path, owner: io_provider::Owner) ->
Result<fs::MutableFilesystem, io::Error>`. The public `fs::system` facade obtains
the current scoped owner before performing that selection operation. A scoped
Veln handler can replace selection without requiring host `fs` effects.

`fs::with_system<effect E>` accepts a native root and a callback
`fn(fs::MutableFilesystem) -> Result<(), io::Error>`. It installs a fresh owner,
the system-selection handler, and the namespace and byte-operation handlers
for that callback. It discharges `fs::Read`, `fs::Mutate`, `io::Read`,
`io::Write`, `io::Seek`, `io::ReadAt`, `io::WriteAt`, `io::Close`, `io::Sync`,
`fs::Host`, and `io_provider::Scope`; it exposes coarse `fs`, `concurrency`,
and the callback's remaining effects in `E`. The callback result is returned
after owner cleanup. An abrupt cleanup failure retains the cleanup precedence
below and is never converted to an ordinary callback result.

`ReadFile` is a record containing `reader: io::Reader`, `seeker: io::Seeker`,
`reader_at: io::ReaderAt`, and `closer: io::Closer`. `WriteFile` contains
`writer: io::Writer`, `seeker: io::Seeker`, `writer_at: io::WriterAt`,
`closer: io::Closer`, `syncer: io::Syncer`, and `truncater: fs::Truncater`.
`UpdateFile` contains all six byte views plus `syncer` and `truncater`.
`AppendFile` contains only `writer`, `closer`, and `syncer`. All fields of
one open refer to the same resource. No view grants a capability absent from
its creation rights. A read-only filesystem cannot be upgraded to mutable.

The type boundary rejects a missing capability. When a provider cannot supply
an entire requested open record, open returns `Unsupported` and releases any
partial host open. It must not publish a partially usable record. Network
adapters publish only reader, writer, and closer views. Optional future
capability discovery does not belong to the first API.

`CreateMode` is `Existing`, `CreateIfMissing`, or `CreateNew`. `WriteMode` and
`UpdateMode` each carry a `CreateMode` and a `truncate: Bool`; `CreateNew` with
truncate is rejected as redundant before host access. Append has a separate
entry point and cannot request read, truncate, seek, or positional-write views.
`SeekOrigin` is `Start(ByteOffset)`, `Current(Int)`, or `End(Int)`.
`FollowLinks` is `Follow` or `NoFollow`.

### Provider-Bound Dispatch Prerequisite

This is an explicit runtime/compiler adapter extension, not a claim that
ordinary nominal dispatch already provides resource-bound routing. Existing
`NetListener` and `NetStream` demonstrate opaque handles, but do not provide a
general Veln-only provider allocation API. Do not reuse their network bridge
or invent an unimplemented trait or closure representation.

The extension supplies these proposed source entry points in `io_provider`:

| Entry point | Required observation |
| --- | --- |
| `Owner`, `Resource` | Opaque identity-bearing types, preserved across task freezing. Only the allocation functions can produce them. |
| `owner() -> Owner effects [io_provider::Scope]` | Returns the owner of the nearest installed `io_provider::scope()` handler. That handler implements nominal effect `Scope`, creates a fresh owner per application, and releases it at handler exit. Ordinary helper returns do not close the owner. No implicit process-global scope. |
| `filesystem(owner: Owner, slot: Int) -> Result<fs::Filesystem, io::Error>` and `mutable_filesystem(owner: Owner, slot: Int) -> Result<fs::MutableFilesystem, io::Error>`, both `effects [concurrency]` | Publish a filesystem bound to the provider's active namespace handlers. Only the mutable factory grants mutation rights; the read-only factory needs no mutation handler. |
| `resource(owner: Owner, slot: Int, family: ViewFamily) -> Result<Resource, io::Error> effects [concurrency]` | Registers one resource with exactly the operations of its declared view family and captures those active handlers, including close. No host file or socket is opened. The provider uses a nonnegative private slot to address its own Veln state. |
| `read_file(resource: Resource)`, `write_file(resource: Resource)`, `update_file(resource: Resource)`, `append_file(resource: Resource)`, `duplex(resource: Resource)` | Return `Result<file-record, io::Error>` of the corresponding file record, or `Result<{reader: io::Reader, writer: io::Writer, closer: io::Closer}, io::Error>` for duplex; these projections are pure; incompatible families fail with `Unsupported`. Missing handlers yield `Unsupported`; no host fallback. |
| `slot(owner: Owner, resource: Resource) -> Result<Int, io::Error> effects [concurrency]` | Returns the private slot only for the matching owner while usable or while its registered close handler is releasing it; wrong-owner access yields `InvalidResource`. Integer equality is never a public resource identity test. |

`ViewFamily` is `ReadFile`, `WriteFile`, `UpdateFile`, `AppendFile`, or
`Duplex`. Registration requires every handler in that family, grants no other
operations, and rejects missing operations before publication. Filesystem
slots have corresponding accessors `filesystem_slot(owner: Owner, filesystem: fs::Filesystem) ->
Result<Int, io::Error> effects [concurrency]` and
`mutable_filesystem_slot(owner: Owner, filesystem: fs::MutableFilesystem) ->
Result<Int, io::Error> effects [concurrency]`, with the same owner checks as
`slot`. Namespace tokens cannot be used as byte resource tokens.
Operation handlers receive capability views, not the underlying `Resource`.
Provider-only slot access therefore also uses `reader_slot(owner: Owner,
reader: io::Reader) -> Result<Int, io::Error> effects [concurrency]` and
corresponding `writer_slot`, `seeker_slot`, `reader_at_slot`, `writer_at_slot`,
`closer_slot`, `syncer_slot`, and `truncater_slot` functions, each taking its
matching nominal view. All enforce the same owner check. There is no public
view-to-Resource conversion that could upgrade a restricted capability.

The same runtime extension supplies `State<T>` and two operations:

- `state<T>(initial: T) -> State<T> effects [concurrency]` creates independent
  synchronized storage; task freezing preserves cell identity.
- `update<T, C, R>(state: State<T>, context: C, action: fn(T, C) -> {state: T,
  result: R}) -> R effects [concurrency]` runs a pure callback under exclusive
  access and commits its returned replacement with its result. Runtime failure
  leaves the old state and releases the lock. Cancellation is delivered before
  entry or after commit, never between acquiring and releasing the state.
  Reentrant updates are rejected as runtime failures rather than deadlock.

This is an explicit compiler-known atomic storage adapter, not an assumed
existing source API. A raw channel receive/compute/send sequence is rejected because
cancellation can strand the state token. Pure callback transitions keep all
fake filesystem decisions in Veln and cannot perform nested provider IO while
holding the cell. Cell allocation and updates expose only `concurrency`, not
host `fs` or `net` effects.

Public filesystem and byte facades invoke the resource's captured operation
handler with its original provider context. They retain the declared nominal
effect in static checking. Runtime routing is restricted to these standard
resource facades; ordinary `perform` keeps lexical dispatch. Calling another
provider's low-level operation with a foreign token fails before changing
provider state. Nested handlers affect newly created resources, while calls on
existing resources retain their creator. Capture does not retain a usable
resource after its owner closes.

A provider can create distinct filesystem slots under one owner. Resource
slots are unique within that owner's state; runtime allocation does not expose
an integer-to-handle lookup or let another owner forge its tokens. Publication
and the owner's closing transition share an atomic admission boundary. A late
host open is closed before returning `Closed`; cleanup failure is retained as
related context and the owner retains its release obligation.

The Veln fake allocates a fresh state cell, installs an owner scope, and
installs its namespace and byte handlers with that cell and owner as context.
Every operation applies a pure state transition through `update`. File
identity is separate from directory binding and per-open slot identity, so
rename cannot redirect an existing handle. Scripted failures, bytes, and logs
are ordinary Veln data. Independent executions allocate separate cells and
owners. Runtime tests must demonstrate cancellation atomicity of `update`
before the source fake relies on it.

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
| `io::CopyOutcome` | `Copied(count: ByteCount)`, `CopyFailed(consumed: Progress, committed: Progress, error: io::Error)`. Consumed and committed totals are separate even when their values happen to match. |
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
| Zero-size read/write/exact request on a live permitted view | Return empty `Data`, `Written(0)`, or empty `Complete` without a provider transfer. Validate owner, capability, and closed state first; zero size cannot revive a closed resource. |
| Copy with zero chunk size | `InvalidInput`; no read or write. |
| Positive request, empty successful read or zero successful write | Direct facade reports `NoProgress`; complete helpers stop. No spin, invented end, or success. |
| Short data read | Deliver bytes and continue only if a helper needs more. It is not end. |
| Read failure with delivered bytes | Include those bytes and consumption in the failure. `read_exact` retains all previously delivered bytes. Copy does not write the failing read's bytes; its consumed total still includes them. |
| Partial successful write | `write_all` retries the suffix. A later failure includes total known committed progress and stops. |
| Exception after a host transfer attempt | Preserve independently established prefix/progress; otherwise report unknown progress. Sequential cursor becomes unavailable until successful absolute `Start` seek or close if its value is unknown. Relative seek fails with `InvalidResource` in that state. |
| Positional failure | Preserve the sequential cursor, including when transfer progress is unknown. Unknown content changes remain observable; no seek-and-transfer emulation. |
| Count outside the submitted bounds or counter overflow | Stop with `InvalidInput` for caller input or `InvalidResource` for an invalid provider reply. Do not wrap counts or retry using a fabricated suffix. |

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
| Close fails with release `No` | Preserve the prior admission state and cleanup obligation; explicit retry is permitted while the owner is active. |
| Close fails with release `Unknown` | Unavailable for transfers, retain release obligation, permit explicit close retry while the owner is active. Never fall back to another provider. |
| Confirmed closed resource; close after owner exit | Return `Closed` without invoking the retained provider or a newer lexical provider. |
| Cancellation during an admitted host call | Preserve proven progress; interruption can close the entire open and all its views. Mark release/cursor facts unknown when unproved. No bounded interruption latency or rollback guarantee. |
| Owner starts cleanup | Close publication and new close admission first. Account for already admitted opens and closes, then attempt retained resources in reverse publication order, even after one close fails. A confirmed Closed/Yes close needs no new release; a No/Unknown close retains its cleanup obligation. Children retain identity but cannot publish or extend ownership. |
| Cleanup has unsuccessful or unknown release | Raise an abrupt cleanup failure after attempting all resources. An admitted open or close still pending when cleanup cannot confirm release is also a cleanup failure; a late open completion must still attempt release. Keep failed release obligations until final owner teardown; never report successful release or transfer them to a different invocation. External termination can still leak host resources under existing cleanup limits. |
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

### Feasibility And Conformance Gate

The provider extension is a named prerequisite, not an unresolved choice of
representation. Before this entire proposal moves to Ready, check a small
source/adapter prototype with these independently specified observations:

| Gate | Required observation and planned evidence |
| --- | --- |
| F1: static source API | All declared file view records, nominal operations, and storage callbacks check using existing syntax plus the named compiler-known types. Wrong capability arguments fail. Provider factories return distinct read-only/mutable types and registrations enforce view families. Compiler type-boundary evidence. |
| F2: Veln-only allocation | Two fake filesystem instances return opaque resources using Veln handler state without any host fs/net call. Nested shadowing does not redirect an older handle; cross-provider copy transfers the selected bytes. Companion tests and a backend dispatch test. |
| F3: state and publication | Concurrent children, injected failure, and cancellation cannot strand the state cell lock or publish half a transition. Closing the owner rejects late publication and releases the late host resource. Scoped Veln support and backend lifecycle evidence. |
| F4: transfer and mutation | Every outcome and transition row above has independent expected bytes, cursor, commit, and namespace observations, including unsupported rename and unknown host progress. Shared companion conformance tests plus focused real-host cases. |
| F5: cleanup | Close failure facts, retry, reverse cleanup, primary/related failures, and owner exit with retained child aliases obey the cleanup table. Companion tests and backend cleanup projections. |

If F1–F3 cannot be demonstrated with the stated adapter extension, revise the
resource design before implementation selection. Do not hide that prerequisite
inside a platform-only fake, or promote a source API that silently assumes
unimplemented dispatch or state primitives.

### Gate Progress

F2, F3, and F5 pass. F1 and F4 remain open. The checked prototype covers storage,
provider-bound filesystem and byte-resource adapters, pure native path conversion,
a Veln memory filesystem, complete-transfer helpers, and an initial real-host
system provider. The fake still needs scripted operation failures. Network
adapters and the remaining host/race conformance evidence are absent.

| Gate | Checked prototype evidence | Remaining evidence before passage |
| --- | --- | --- |
| F1 | Compiler integration checks in `crates/veln-analysis/src/tests/io_provider.rs` accept the four file-view record shapes, native-path API, host-selection boundary, and system-scope helper with retained callback effects. They reject reader-to-writer, stream-to-seeker, read-only-to-mutable, integer-to-owner, and portable-to-native-path/root arguments. A companion test injects filesystem selection without host fs effects. An application handler checks error records containing portable paths. Explicit channel and task type arguments resolve imported capability, record, and storage types before checking. State/update aliases retain linked inferred and explicit types; backend execution distinguishes the selected standard import from a same-spelled application module. Adapter type checks in `crates/veln-sema/src/tests/standard_library_effects/io_provider.rs` cover state/context/result linking and callback purity. | Check the network adapter surface. The shared helpers already execute under nominal byte handlers without host effects. |
| F2 | `crates/veln-stdlib/veln/fs.test.veln` allocates two Veln memory filesystems and copies selected bytes through their namespace/open boundaries under lexical shadowing. The support modules use scoped Veln handlers and state without host filesystem or network calls. `crates/veln-backend-jvm/src/tests/io_provider_dispatch.rs` checks opaque owner/resource identities through task freezing and an unrelated owner scope ending first. The byte-provider companion tests distinguish captured facade routing from ordinary lexical `perform`. | None for F2. Host resource publication remains part of F3. |
| F3 | The companion tests cover shared child-task state, independent cells, wrong-owner slot access/publication, and aliases retained beyond owner exit. The backend storage harness in `crates/veln-backend-jvm/test-support/IoProviderStateHarness.java` checks callback failure rollback, lock release, reentrancy rejection, and cancellation before entry or after commit. Scoped Veln publication gates in `crates/veln-stdlib/veln/io_publication_test_support_test.veln` hold an admitted open across owner exit. Its companion case checks rejected publication and one actual release; backend projections check late release No/Yes/Unknown against independent live state, and retain a late No failure through runtime failure and cancellation. A still-pending producer causes unconfirmed-cleanup failure, then attempts release when allowed to finish. The real-host harness `crates/veln-backend-jvm/test-support/IoProviderHostPublicationHarness.java` opens actual FileChannels before closing publication and observes their release, including a pending task cancellation at the compiled-handler checkpoint. | None for F3. Explicit-close overlap is checked under F5. |
| F4 | `crates/veln-stdlib/veln/io.test.veln` checks exact reads, complete writes, and copy against scripted bytes, committed prefixes, consumed input, request logs, and failure contexts. Cases include clean end, partial success, known/unknown failure, no progress, zero-size validation, invalid reply counts, and count overflow. The filesystem companion tests observe independent cursors, positional IO, seek failure preservation and overflow, open modes, append, truncate, unlink/rename identity, directory failure preservation, link traversal, scalar-sorted enumeration, and whole-list rejection on injected name conversion failure. Pure native conversion rejects NUL; the JVM host-boundary harness `crates/veln-backend-jvm/test-support/IoProviderNativePathHarness.java` rejects malformed UTF-16 instead of replacing it with another native filename. The real-host backend cases observe selected-root FileChannel reads/writes, truncation, creation, sync, scalar-sorted Unicode entries, non-mutation on unsupported atomic rename, and whole-list failure for an actual unrepresentable Unix native byte name. Unix host cases also observe directory creation/removal, a retained FileChannel reading its original file after unlink and replacement of its binding, and symlink traversal outside the selected root. A mixed host-to-Veln-memory copy preserves selected bytes and captured provider routing. The real FileChannel interruption harness `crates/veln-backend-jvm/test-support/IoProviderHostTransferHarness.java` checks unknown zero-prefix progress, actual channel closure, unchanged disk bytes, and sequential versus positional cursor knowledge.  Scoped gates in `crates/veln-stdlib/veln/io_admission_test_support_test.veln` hold each transfer, seek, truncate, and sync operation while probing read, write, positional IO, seek, truncate, and sync admission. Companion cases compare Busy zero-progress outcomes with unchanged bytes, both cursors, and operation logs; they also observe independent-open progress, recovered admission, and cleanup order. The Unix native-descriptor harness `crates/veln-backend-jvm/test-support/IoProviderHostCloseFailureHarness.java` observes a still-readable descriptor after failed close and logical channel closure, preserves Unknown release across retries, and blocks transfer aliases. Recoverable native transfer errors invalidate only sequential cursor knowledge; positional failures preserve it, positional successes leave an unknown cursor unknown, and absolute seek restores it. Failed native seek/sync and pre-truncate size lookup preserve position and bytes. | Cover scripted filesystem failures with positive transfer prefixes and committed namespace changes, close/transfer interleavings, network context, and remaining host namespace failure classifications. |
| F5 | The provider companion tests check close failure with release No/Yes/Unknown against independent live-resource state, retry, transfer admission, and repeated close without another provider call. They also check that a retry reporting No cannot clear earlier Unknown release state, reverse owner cleanup, and a child retaining an alias until after owner exit. The backend provider tests observe every close callback after multiple release failures, preserve their order in human output and diagnostic traces, keep existing runtime/contract failures primary, and retain an ordinary Err as related context after normal return or postfix propagation. A gated child-task case observes every close callback before cancellation reaches join and retains ordered release failures in its diagnostic trace. Scoped overlap support in `crates/veln-stdlib/veln/io_close_overlap_test_support_test.veln` gates an explicit close across owner exit. Companion cases check Busy for a second active close, blocked reads, original No/Yes/Unknown outcomes, cleanup retry, cancellation after release, and idempotent close after owner exit without provider fallback. Backend projections additionally check cleanup retry failures with No/Yes/Unknown, still-pending close failure, and retry after runtime/contract failure in the closer itself against independent close events and live state. | None for F5. |

The backend checks in
`crates/veln-backend-jvm/src/tests/deferred_cleanup.rs` exercise `?`
propagation before and after nested handlers. The `deferred_cleanup` filter
also checks cleanup order, failure precedence, and bytecode growth. These
checks support compiler integration; they do not replace the provider-specific
cleanup cases required by F5.

Run `veln test fs.test.veln io.test.veln io_provider.test.veln` from the standard-library package
after rebuilding the CLI against the current standard-library sources.
Run the compiler and backend checks through `scripts/agent-test` with the
`io_provider` filter. These checks cover the observations named above, not
the complete filesystem contract.

## Implementation Readiness And Promotion

The initial design decisions and acceptance tables are fixed above. Complete
the provider-bound dispatch feasibility gate before moving this proposal to
Ready. The adapter extension must be reviewed together with its source API,
not inferred from the existing network handles. A smaller independently complete
proposal may be split out only when it retains explicit ownership and outcome
contracts.

When implementation lands, update the smallest current specifications for
effects, shared IO, filesystem operations, and execution ownership alongside
the checked evidence. Add documentation routes only for implemented surfaces.
Remove implemented sections from this proposal, and remove the page and its
catalog entry when no unimplemented target remains.
