---
role: proposal
update-when: The proposed filesystem path model, shared byte IO capabilities, provider ownership, seek behavior, fake filesystem contracts, or unresolved design decisions change.
---

# Filesystem And Shared Byte IO

Provide practical filesystem access with streaming binary IO, random access,
portable paths, and scoped substitution through Veln effect handlers. The same
byte-processing helpers should work with files, network streams, and memory.

This proposal fixes the contracts described below and records unresolved
choices in [Open Questions](#open-questions). It is not ready for implementation
selection until those questions have concrete answers for the initial scope.
All acceptance evidence is planned. Operation names in tables are conceptual;
they do not declare a complete source API or introduce language syntax.

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
| Portable paths and explicit filesystem selection | Application names have the same syntax under host and fake providers. |
| Read-only filesystem access | Open, metadata, directory enumeration, and byte reads work without requiring mutation capability. |
| Filesystem updates | Explicit open modes, byte writes, directory creation, removal, rename, and truncation have defined success and failure behavior. |
| Shared byte IO | Read, write, complete-transfer helpers, seek, and positional IO compose with explicit resources. |
| Resource lifecycle | Close and scope exit preserve provider identity and prevent accidental access to another provider. |
| Deterministic testing | A Veln fake implements stateful filesystem behavior and injected failures without opening host files. |

Memory mapping, file watching, advisory locks, hard links, detailed ACLs,
distributed filesystem consistency, crash transactions, and asynchronous IO
scheduling are outside the initial scope. Symbolic-link traversal and native
filename representation still need the decisions identified below.

## Paths And Filesystem Selection

Use distinct types for a portable filesystem path and a native host path.
The portable type names a location within a selected filesystem. It does not
identify that filesystem, authorize access, or assert that an entry exists.
The existing host-owned `Path` must not silently acquire portable semantics.
Final public type names and migration of `process::cwd()` remain open.

Portable paths use case-sensitive Unicode text with `/` as the only separator.
Path equality compares the stored text exactly; it does not case-fold,
normalize Unicode, resolve links, or establish file identity. A host provider
can therefore resolve two unequal names to the same entry on a host whose
lookup rules equate them.

The fallible portable-path constructor validates without accessing a provider:

| Input | Result |
| --- | --- |
| `.` | Root of the selected filesystem. |
| `a`, `a/b`, or a Unicode filename | Valid portable path. |
| Empty text, leading or trailing `/`, or repeated `/` | Invalid path. |
| A `.` or `..` component, except the complete root spelling `.` | Invalid path. |
| Embedded NUL | Invalid path. |
| Backslash or colon within a component | Ordinary portable name characters; a host conversion may reject them but must not reinterpret them as separators or a volume prefix. |

Construction rejects invalid spelling instead of silently cleaning it. Pure
composition and component operations preserve these invariants. A join with
root returns the other operand; joining two non-root paths inserts one `/`.
The parent of root is absent; the parent of a single component is root.

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

Go's [io/fs path contract](https://pkg.go.dev/io/fs#ValidPath) motivates the
portable syntax and small path vocabulary. Its
[filepath.Localize](https://pkg.go.dev/path/filepath#Localize) demonstrates why
portable validity and native representability are different checks. Veln adds
a validated path type rather than accepting unchecked strings at every call.
Rust's [native Path conversion](https://doc.rust-lang.org/std/path/struct.Path.html#method.to_str)
shows why an arbitrary native name need not have a lossless Unicode spelling;
the native representation decision remains open rather than assuming all host
names are portable.

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
of the newly active provider. The precise dispatch mechanism and static
capability representation are Open Questions.

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
and network adapters retain domain context when translating failures. Exact
effect names, error declarations, and relationships to `transport` remain open.

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
failure with unknown progress. Zero-size behavior and the outcome ADTs remain
Open Questions.

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
| Resource has no seek capability | Not accepted by a static capability boundary, or explicit unsupported failure where support is dynamic. The exact split remains open. |
| Closed resource | Typed closed-resource failure; no transfer or cursor change. |
| Positional read/write | Sequential cursor is unchanged, including after an operation that reports partial progress. |

Each independent open has its own cursor and close state. Copying a handle
refers to the same open resource and shares those states. Two opens of one
file refer to the same underlying contents, subject to the provider's stated
external-consistency limits. They do not share cursors merely because their
paths are equal.

Rust's [Seek](https://doc.rust-lang.org/std/io/trait.Seek.html) motivates a
typed origin rather than integer `whence`. This proposal additionally fixes
invalid-target state preservation and seeking beyond end. The contents of a
gap created by a later write, append interactions, buffering, and failed host
seek state still require decisions.

## Filesystem Updates And Resource Lifetime

Opening distinguishes access rights, creation, exclusive creation, truncation,
and append behavior explicitly. Invalid combinations fail before modifying the
filesystem. Read-only open does not create or truncate. Exclusive creation
does not use an existence check followed by a separate create; when the entry
already exists, its contents remain unchanged.

Metadata identifies at least file kind and byte length where meaningful.
Directory entries carry their names and kind information rather than only
repeating complete paths. Directory enumeration must state its ordering and
its behavior if entries change during enumeration; those choices remain open.
Missing metadata and unsupported operations must not be encoded as invented
zero values or successful no-ops.

Creation, removal, rename, truncation, append, and sync need success, failure,
and state-transition tables before implementation selection. In particular,
rename needs a replacement and cross-filesystem policy. A successful byte
write or close does not itself promise crash durability. Buffered flush and
storage synchronization are distinct concepts.

An owner scope releases retained resources on normal return, propagated
errors, runtime failure, and task cancellation within the existing
[execution cleanup limits](../specification/execution.md#cleanup-limits).
Explicit close releases a resource earlier. Closing or leaving its owner
scope makes all aliases unusable; later calls cannot fall back to host IO or
another provider. The owner stops publishing new resources when scope cleanup
starts. A child operation completing an open after that point releases the new
resource instead of publishing a usable handle.

Child tasks inherit the creating scope's handlers and resource ownership;
they do not extend a parent's owner lifetime merely by retaining a handle.
Independent executions construct independent fake state. Error precedence,
failed-close ownership, and the result type of a scoped file helper remain
Open Questions. Existing `defer` cannot silently discard a meaningful close
failure and is not a substitute for specifying these outcomes.

Java's [FileChannel](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/nio/channels/FileChannel.html)
supports exclusive creation and positional IO, but leaves append atomicity
and gap contents dependent on the host contract. Its
[seekable channel boundary](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/nio/channels/SeekableByteChannel.html)
also permits interruption to close a channel. These are backend constraints
to resolve explicitly, not guarantees supplied by a portable fake.

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

All evidence below is planned. Expected byte sequences and state transitions
come from the contracts above and the completed Open Questions, not from
copying the implementation's output. Fake tests do not establish real host
behavior.

| Concern | Observable acceptance | Planned evidence |
| --- | --- | --- |
| Portable paths | The validity table holds; pure composition preserves it; case and Unicode spelling are not silently changed. | Table-driven standard-library companion tests and path doctests. |
| Host conversion | An unrepresentable name fails without selecting another entry; native and portable types cannot be interchanged implicitly. | Companion tests plus focused host-conversion and compiler type-boundary tests. |
| Multiple resources | Copy between two resources and two filesystem instances reaches the intended input and output under nested handlers. | Companion tests with distinct bytes and provider traces. |
| Partial transfer | Short reads are data; complete helpers preserve bytes and progress; partial writes retry only an uncommitted suffix; unknown progress stops retry. | Scripted Veln fake tests with independent expected contents and operation logs. |
| Seek and positional IO | Origin arithmetic, invalid targets, beyond-end reads, independent opens, aliases, and positional cursor preservation obey the state table. | Companion tests shared by memory and supported system providers; host tests for native translation. |
| Namespace updates | Open modes, exclusive create, directory operations, rename, and truncation obey completed transition tables, including failure preservation. | Shared provider conformance tests and focused real-host integration cases. |
| Ownership | Nested scopes do not redirect handles; scope exit closes aliases; child tasks cannot publish into a closing owner. Cleanup runs during error and cancellation unwinding. | Veln companion tests for provider behavior and JVM owner/handler integration tests. |
| Errors and cleanup | Stable error kinds retain operation and path context; completed close and cleanup rules preserve progress and the selected primary failure. | Companion tests plus human/JSON execution coverage where runtime cleanup diagnostics apply. |
| Real host default | A system handler reaches real files without fake selectors, while an injected fake requires no host-file setup. | Focused host integration and effect-boundary policy checks. |

Standard-library API behavior belongs in `crates/veln-stdlib/veln/` companion
tests. Compiler, backend, and host integration cases must identify their own
contract under the [test placement policy](../reference/toolchain-test-harness.md).
No broad stress or performance claim is part of this proposal.

## Open Questions

Each question blocks implementation readiness for its affected capability.
Resolving one requires updating the relevant contract and acceptance row;
listing a preferred direction here does not make it implemented behavior.

| Question | Alternatives and required decision |
| --- | --- |
| Public source API and static capabilities | Choose final modules, types, operation signatures, and nominal effects. Decide how read-only, writable, seekable, and positional capabilities are represented with the existing type system. Avoid assuming an unimplemented trait or effect feature. |
| Provider dispatch and fake-created handles | Choose a provider-bound resource representation and dispatch rule that supports multiple providers, nested handlers, and Veln-only fake creation. Define wrong-provider rejection and identity without exposing host objects or forgeable integer handles. |
| State for Veln fakes | Decide whether existing synchronized channel-backed state is sufficient or a separate state abstraction is required. Specify initialization, child-task sharing, and provider resource allocation. |
| Native paths and the old boundary | Decide whether arbitrary native names round-trip, how display differs from lossless conversion, where host roots are opened, and how `process::cwd()` and compiler-known `fs` names change. Backward compatibility is not required. |
| Links and confinement | Define follow/no-follow behavior for open and metadata, link cycles and dangling links, subtree views, and whether an enforceable confinement capability belongs in the first implementation. State unsupported backend outcomes. |
| IO outcomes and error translation | Define common read/write/copy ADTs, zero-size and zero-progress rules, partial data plus failure, unknown progress, and stable error categories. Specify how filesystem and network context survives adaptation and what a host exception can actually prove. |
| Remaining seek and mutation transitions | Define append with seek or positional write, writes beyond end and gap bytes, truncate cursor behavior, failed host seek state, metadata fields, directory ordering/pagination, remove semantics, and rename replacement/cross-filesystem outcomes. |
| Concurrent operations and cancellation | Decide serialization versus busy errors for shared cursors, positional-call concurrency, close races, and task interruption outcomes. Do not infer bounded cancellation from synchronous JVM file calls. |
| Cleanup and durability | Define explicit and repeated close, retry after failed close, all-resource cleanup after one close fails, primary/related error precedence, scoped helper results, flush versus sync, and the supported durability guarantee. Compare with existing network cleanup before choosing different behavior. |
| Existing transport and buffering | Decide how `transport::DuplexStream` and network errors adapt to shared IO without forcing file operations to expose `net::IO`. Define buffer ownership, flush ordering, and seek invalidation before adding buffered helpers. |

## Implementation Readiness And Promotion

Before moving this proposal to Ready, provide a source API that can be expressed
with implemented Veln features or name a separate language prerequisite.
Complete the outcome types and operation state tables, including failures,
and demonstrate how a Veln fake creates resources and serves two filesystem
instances without host IO. Add explicit acceptance expectations for each
resolved question. A smaller independently complete proposal may be split out
instead of treating unresolved sections as implementation instructions.

When implementation lands, update the smallest current specifications for
effects, shared IO, filesystem operations, and execution ownership alongside
the checked evidence. Add documentation routes only for implemented surfaces.
Remove implemented sections from this proposal, and remove the page and its
catalog entry when no unimplemented target remains.
