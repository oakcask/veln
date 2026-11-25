---
role: specification
authority: normative
specification-coverage: usage=#imports-and-static-surface; behavior=#runtime-prototypes; limits=#runtime-limits
update-when: The exported io, fs, or io_provider static source API, direct byte-transfer request or reply outcomes, opaque capability identities, atomic provider-state typing or runtime behavior, provider-bound dispatch or owner-lifecycle behavior, checked filesystem IO evidence, file-view declarations, or filesystem IO runtime readiness changes.
---

# Standard-library filesystem and byte IO

The standard package exports `io`, `fs`, and `io_provider`. Their current
contract combines a compiler-checked static surface with bounded `ReadFile`,
`WriteFile`, `UpdateFile`, `AppendFile`, and `Duplex` registration and
projection prototypes, provider-bound filesystem namespace tokens, and opaque
atomic provider state. It lets provider and generic IO source check
capability-correct signatures, capture and invoke provider handlers, exercise
five runtime resource families, transfer bytes between independently owned
fake resources, validate direct byte-transfer replies, complete bounded reads
and writes, exercise per-open cursors, seek-beyond-end sequential extension,
and positional IO in a scoped fake, exercise a bounded JVM rooted-host
write after seeking beyond end, publish its rooted namespace token and
host-backed write resource without replacing concurrent provider
registrations, keep
immutable fake-provider state in shared cells, and retain
completed fake-provider writes from concurrent child callers. Provider owner
exit closes publication before release, rejects and releases a resource whose
provider call finishes after that boundary, and restricts cleanup access to the
matching owner and exact resource being released.
It is not a production filesystem API.

## Imports and static surface

Import the modules explicitly:

```veln
use fs from "std"
use io from "std"
use io_provider from "std"
```

The byte facades and their nominal effects are:

| Effect | Facade signature |
| --- | --- |
| `io::Read` | `read(Reader, ByteCount) -> ReadOutcome` |
| `io::Write` | `write(Writer, ByteChunk) -> WriteOutcome` |
| `io::Seek` | `seek(Seeker, SeekOrigin) -> Result<ByteOffset, Error>` |
| `io::ReadAt` | `read_at(ReaderAt, ByteOffset, ByteCount) -> ReadOutcome` |
| `io::WriteAt` | `write_at(WriterAt, ByteOffset, ByteChunk) -> WriteOutcome` |
| `io::Close` | `close(Closer) -> CloseOutcome` |
| `io::Sync` | `sync(Syncer) -> Result<(), Error>` |

`read_exact`, `write_all`, and `copy` retain `io::Read`, `io::Write`, or both
in their declared effect rows. Their result types represent partial data,
progress, and error alternatives through `ReadOutcome`, `WriteOutcome`,
`ExactOutcome`, and `CopyOutcome`.

The complete-transfer facade signatures are:

| Function | Signature |
| --- | --- |
| `io::read_exact` | `(Reader, ByteCount) -> ExactOutcome effects [io::Read]` |
| `io::write_all` | `(Writer, ByteChunk) -> WriteOutcome effects [io::Write]` |
| `io::copy` | `(Reader, Writer, ByteCount) -> CopyOutcome effects [io::Read, io::Write]` |

The static outcome types preserve partial data and progress explicitly:

| Type | Variants |
| --- | --- |
| `Progress` | `Known(ByteCount)`, `Unknown(ByteCount)` |
| `Commit` | `No`, `Yes`, `Unknown` |
| `ReadOutcome` | `Data(ByteChunk)`, `End`, `ReadFailed(ByteChunk, Progress, Error)` |
| `WriteOutcome` | `Written(ByteCount)`, `WriteFailed(Progress, Error)` |
| `ExactOutcome` | `Complete(ByteChunk)`, `Short(ByteChunk)`, `ExactFailed(ByteChunk, Progress, Error)` |
| `CopyOutcome` | `Copied(ByteCount)`, `CopyFailed(Progress, Progress, Error)` |
| `CloseOutcome` | `Closed`, `CloseFailed(Commit, Error)` |
| `SeekOrigin` | `Start(ByteOffset)`, `Current(Int)`, `End(Int)` |

`Error` retains an operation, `ErrorKind`, optional primary and secondary
portable paths, optional network error, optional commit fact, and a message.
`ErrorKind` can be `InvalidInput`, `NotFound`, `AlreadyExists`,
`PermissionDenied`, `NotDirectory`, `IsDirectory`, `DirectoryNotEmpty`,
`LinkLoop`, `UnrepresentablePath`, `CrossFilesystem`, `Unsupported`, `Closed`,
`Busy`, `InvalidResource`, `Cancelled`, `NoProgress`, or `Other`. Captured
Veln handlers can produce the individual operation outcomes. `read_exact`,
`write_all`, and bounded `copy` execute through captured provider handlers and
preserve their documented partial outcomes. The
[runtime limits](#runtime-limits) identify the remaining mutation, cleanup,
and host-translation behavior.

`fs::Read` and `fs::Mutate` separate namespace reads from mutation. Portable
names use `path::Path`. Their facade signatures are:

| Effect | Facade signature |
| --- | --- |
| `fs::Read` | `open_read(Filesystem, path::Path) -> Result<ReadFile, io::Error>` |
| `fs::Read` | `metadata(Filesystem, path::Path, FollowLinks) -> Result<Metadata, io::Error>` |
| `fs::Read` | `entries(Filesystem, path::Path) -> Result<List<Entry>, io::Error>` |
| `fs::Mutate` | `open_write(MutableFilesystem, path::Path, WriteMode) -> Result<WriteFile, io::Error>` |
| `fs::Mutate` | `open_read_write(MutableFilesystem, path::Path, UpdateMode) -> Result<UpdateFile, io::Error>` |
| `fs::Mutate` | `open_append(MutableFilesystem, path::Path, CreateMode) -> Result<AppendFile, io::Error>` |
| `fs::Mutate` | `create_directory(MutableFilesystem, path::Path) -> MutationOutcome` |
| `fs::Mutate` | `remove_file(MutableFilesystem, path::Path) -> MutationOutcome` |
| `fs::Mutate` | `remove_directory(MutableFilesystem, path::Path) -> MutationOutcome` |
| `fs::Mutate` | `rename(MutableFilesystem, path::Path, path::Path) -> MutationOutcome` |
| `fs::Mutate` | `truncate(Truncater, ByteOffset) -> MutationOutcome` |

`CreateMode` is `Existing`, `CreateIfMissing`, or `CreateNew`. `WriteMode` and
`UpdateMode` each contain a `CreateMode` and a `Bool` truncate flag.
`FollowLinks` is `Follow` or `NoFollow`. `MutationOutcome` is `Applied` or
`MutationFailed(io::Commit, io::Error)`. `Metadata` contains a `Kind` and an
optional byte length. `Entry` contains a portable name and a `Kind`; `Kind` is
`RegularFile`, `Directory`, `SymbolicLink`, or `Other`.

Host selection uses
`fs::system(Path) -> Result<MutableFilesystem, io::Error> effects [fs::Host,
io_provider::Scope]`. `fs::system_handler()` discharges `fs::Host` and retains
coarse `fs` plus `concurrency`. `fs::read_only(MutableFilesystem) ->
Filesystem` is the only static projection from a mutable filesystem token to a
read-only token.

The file-view declarations carry these capability fields:

| View | Capabilities |
| --- | --- |
| `ReadFile` | reader, seeker, positional reader, closer |
| `WriteFile` | writer, seeker, positional writer, closer, syncer, truncater |
| `UpdateFile` | reader, writer, seeker, both positional views, closer, syncer, truncater |
| `AppendFile` | writer, closer, syncer |

They are single-variant public ADTs because Veln has no named structural-record
alias syntax. Their constructor payload names preserve the file-view field
contract. Possessing those file-view values does not create or upgrade any
opaque capability value.

`io_provider` exposes the adapter surface below. State allocation and update,
the factories, and both slot groups retain `concurrency`; `owner` requires
`Scope`, the `scope` handler retains `concurrency`, and the resource
projections are pure.

| Group | Signatures |
| --- | --- |
| Atomic state | `state(initial: T) -> State<T> effects [concurrency]`; `update(cell: State<T>, context: C, action: fn(T, C) -> {state: T, result: R}) -> R effects [concurrency]` |
| Scope and factories | `owner() -> Owner effects [Scope]`; `scope()` handles `Scope` with `concurrency`; `filesystem(Owner, Int) -> Result<fs::Filesystem, io::Error>`; `mutable_filesystem(Owner, Int) -> Result<fs::MutableFilesystem, io::Error>`; `resource(Owner, Int, ViewFamily) -> Result<Resource, io::Error>` |
| Resource projections | `read_file(Resource) -> Result<fs::ReadFile, io::Error>`; `write_file(Resource) -> Result<fs::WriteFile, io::Error>`; `update_file(Resource) -> Result<fs::UpdateFile, io::Error>`; `append_file(Resource) -> Result<fs::AppendFile, io::Error>`; `duplex(Resource) -> Result<{reader: io::Reader, writer: io::Writer, closer: io::Closer}, io::Error>` |
| Identity slots | `slot(Owner, Resource) -> Result<Int, io::Error>`; `filesystem_slot(Owner, fs::Filesystem) -> Result<Int, io::Error>`; `mutable_filesystem_slot(Owner, fs::MutableFilesystem) -> Result<Int, io::Error>` |
| Capability slots | `reader_slot(Owner, io::Reader)`, `writer_slot(Owner, io::Writer)`, `seeker_slot(Owner, io::Seeker)`, `reader_at_slot(Owner, io::ReaderAt)`, `writer_at_slot(Owner, io::WriterAt)`, `closer_slot(Owner, io::Closer)`, `syncer_slot(Owner, io::Syncer)`, and `truncater_slot(Owner, fs::Truncater)` each return `Result<Int, io::Error>` |

`ViewFamily` is `ReadFile`, `WriteFile`, `UpdateFile`, `AppendFile`, or
`Duplex`. The registration selector does not add capabilities by itself.
The atomic state behavior and the bounded registration and projection behavior
described below are implemented.

The compiler adapters behind these declarations are available only to source
embedded in the standard package. Application source cannot call their
reserved `prelude_builtin` names, including by changing the name casing. Each
reserved adapter is an implementation boundary, not a separately callable
source API. The public declarations above own the fixed operand and result
types. Generic state wrapper calls retain their inferred `T`, `C`, and `R` at
that boundary. This specialization requires the standard package identity;
same-spelled declarations from another package retain their own declared call
types through checked core and typed IR. An expected type at a call site cannot
reinterpret an owner as a byte capability or replace a required owner operand
with an integer.

## Capability boundaries

`Owner`, `Resource`, `Filesystem`, `MutableFilesystem`, `Reader`, `Writer`,
`Seeker`, `ReaderAt`, `WriterAt`, `Closer`, `Syncer`, and `Truncater` are
distinct opaque nominal types. Source has no constructor, numeric cast,
host-object conversion, variant, or refinement for any of them. Public aliases
retain the compiler-owned identity instead of creating a source representation.

Consequently, a writer cannot satisfy a reader operand, a closer cannot
satisfy a seeker operand, and a read-only filesystem cannot satisfy a mutation
operand. `io_provider::filesystem` and `mutable_filesystem` return distinct
result types. File-view projections also retain their declared result type;
one projection cannot be assigned as another file-view family. The focused
cases under [References](#references) check wrong and missing operands,
filesystem and projection separation, and opaque identities.

## Runtime prototypes

### Atomic state

`io_provider::state` creates an independent opaque `State<T>` cell. Passing a
cell to a child task preserves cell identity, so parent and child updates share
one transition sequence. `State<T>` has no source constructor or pattern, so
application source and an exact `io_provider.test.veln` companion cannot forge
or inspect a cell; `io_provider::state` is the only allocation path. A concrete
expected `State<T>` supplies `T` to the
initial value expression, so an annotation can resolve an otherwise ambiguous
nullary generic constructor such as `None`. Without a concrete expected state
type or an independently concrete initial value, allocation remains ambiguous.
List, `if`, and `match` initializers retain the joined concrete type of
compatible contributions through checked core and typed IR. Nested compatible
named arguments and same-field records remain concrete when record field order
differs. Record-width joins retain the unique contributed record accepted by
every contribution. Source-ADT arguments apply their checked variance: a
contravariant parameter can select a bounded meet, while incompatible
invariant arguments fail before checked core or typed IR is produced. Neither
join result depends on contribution order. State allocation in an ordinary
function or a handler operation clause retains the resulting concrete
`State<T>` type through checked core and typed IR. The cell's nominal
standard-library identity
and invariant `T` prevent a same-spelled source type or a state type whose
record argument omits retained fields from satisfying the state API's cell
boundary. The
[type rules](types.md#inference-rules) own these inference and compatibility
rules.

`io_provider::update` admits one callback at a time, passes it the current value
and supplied context, and publishes the returned replacement and result as one
transition. Its callback is pure. Attempting to update the same cell reentrantly
fails at runtime instead of waiting on itself.

When several updates wait for one cell, each completed, failed, or cancelled
admitted update releases at most one queued successor. A cancelled queued
update is removed before its callback runs and does not prevent a later
eligible update from entering. A queued callback that fails also releases the
next eligible update, while preserving the old value.

Cancellation before admission does not invoke the callback and is reported as
task cancellation. Cancellation at a generated checkpoint after admission
aborts the callback, preserves the old value, and is reported as task
cancellation, including for a compiled tail-recursive callback. Callback or
runtime failure before publication preserves the old value and propagates the
failure to the caller. If the callback returns before cancellation is observed,
the replacement is published and the cell is released before cancellation is
reported; the cancelled task does not receive the callback result. Every
outcome releases admission bookkeeping.
Cancellation remains cooperative when a callback neither returns, fails, nor
reaches a generated checkpoint. State values, callback context, replacements,
and results can contain deeply nested Veln lists without consuming JVM
call-stack depth in proportion to the list depth. A completed deep-value
transition leaves the cell available for a later update.

### Filesystem namespace tokens

Within the nearest active `io_provider::scope`,
`io_provider::filesystem(owner, slot)` captures the active `fs::Read` handler
and returns an opaque read-only filesystem token. The mutable factory captures
both the active `fs::Read` and `fs::Mutate` handlers and returns an opaque
mutable token. Neither factory invokes a captured handler or falls back to a
host filesystem or network effect.

Both factories require the supplied owner to match the nearest active provider
scope. They accept a nonnegative private slot that is unused by any namespace
token or byte resource registered to that owner. An owner from another,
inactive, or closed scope and a negative or duplicate slot return
`io::ErrorKind::InvalidInput` before handler resolution. Duplicate rejection
leaves the existing registration unchanged. A missing required handler returns
`io::ErrorKind::Unsupported` and leaves the slot available for a later complete
factory call.

The nearest-scope requirement is lexical registration authority. Captured
dispatch does not extend it. If a retained namespace handler runs while a
nested provider scope is nearest, that handler cannot use the dispatched
token's owner to register another namespace token. The attempt returns
`io::ErrorKind::InvalidInput` before resolving a namespace handler, does not
consume the requested slot, and does not change provider state. The outer
owner can use that slot after its scope becomes nearest again.

`fs::read_only` removes mutation authority from a mutable token while retaining
its owner, private slot, and captured read handler. This pure projection does
not require the owner scope to remain active. The matching `filesystem_slot`
and `mutable_filesystem_slot` accessors return the private slot only while the
supplied owner is the matching active owner. Source typing rejects the other
namespace-token kind. Wrong-owner and closed-owner slot access returns
`io::ErrorKind::InvalidResource`.

Scope exit prevents an inherited child task from allocating another namespace
token with that owner or reading an existing token's private slot. It does not
invalidate the pure read-only projection. It closes byte-resource publication
before running the bounded owner-release lifecycle described below.

### File views

Within an active `io_provider::scope`, `io_provider::resource` accepts an
unused nonnegative private slot for `ViewFamily::ReadFile` only when active
Veln handlers provide `io::Read`, `io::Seek`, `io::ReadAt`, and `io::Close`.
It validates the slot before resolving those handlers. A negative slot or a
slot already registered to that owner returns `io::ErrorKind::InvalidInput`;
a duplicate registration does not replace the existing resource. For an unused
nonnegative slot, registration captures the required handlers without invoking
them. If any handler is absent, registration returns
`io::ErrorKind::Unsupported` before publishing the resource. This failure
performs no host filesystem or network fallback and does not consume the slot,
so a complete registration can subsequently use the same owner and slot.

A captured namespace handler can register a byte resource for the owner of
the filesystem token that invoked it, even when another provider scope is
lexically nearest. This invocation authority is limited to that owner and the
duration of the captured call. It does not authorize namespace-token
registration. A call that starts after the owner's scope closes returns
`io::ErrorKind::InvalidResource` before provider invocation. If a captured
namespace call was admitted while the owner was open, its provider call can
finish after closure. The produced resource then fails publication with
`InvalidResource` and is released exactly once. Neither rejection changes an
unrelated provider state cell.

For `ViewFamily::AppendFile`, registration applies the same owner, slot,
validation-before-publication, and duplicate-preservation rules. It requires
active `io::Write`, `io::Close`, and `io::Sync` handlers and captures all three
without invoking them. A missing handler returns `io::ErrorKind::Unsupported`
without consuming the private slot, so a later complete registration can use
that owner and slot.

For `ViewFamily::WriteFile`, registration applies the same validation and
state-preservation rules. It requires active `io::Write`, `io::Seek`,
`io::WriteAt`, `io::Close`, `io::Sync`, and `fs::Mutate` handlers. Registration
captures all six without invoking them. If any handler is absent, registration
returns `io::ErrorKind::Unsupported` without publishing a resource or consuming
the slot. A negative or duplicate slot returns `io::ErrorKind::InvalidInput`.
Neither same-family nor cross-family duplicate registration replaces the
existing resource.

For `ViewFamily::UpdateFile`, registration requires active `io::Read`,
`io::Write`, `io::Seek`, `io::ReadAt`, `io::WriteAt`, `io::Close`, `io::Sync`,
and `fs::Mutate` handlers. It validates the active owner and the nonnegative,
unused slot before resolving handlers. Registration captures all eight
handlers without invoking them and publishes the resource only after the
complete handler set is available. A missing handler returns
`io::ErrorKind::Unsupported`, publishes nothing, and leaves the slot available
for a later complete registration. Negative and duplicate slots return
`io::ErrorKind::InvalidInput`. Same-family duplicates and cross-family
duplicates with `ReadFile`, `WriteFile`, and `AppendFile` preserve the original
resource regardless of registration order.

For `ViewFamily::Duplex`, registration requires active `io::Read`, `io::Write`,
and `io::Close` handlers. It validates the active owner and the nonnegative,
unused slot before resolving handlers. Registration captures all three
handlers without invoking them and publishes the resource only after the
complete set is available. A missing handler returns
`io::ErrorKind::Unsupported` without consuming the slot. A negative or reused
slot returns `io::ErrorKind::InvalidInput` without resolving operation
handlers.

`io_provider::read_file` projects an accepted resource into reader, seeker,
positional-reader, and closer capabilities. The resource and every projected
capability retain the registration owner and private slot, as observed by the
matching slot accessors. Projecting the resource through
`io_provider::update_file` or `io_provider::duplex` returns
`io::ErrorKind::Unsupported`. These failed projections do not alter the
resource, and a later `read_file` projection still succeeds.

`io_provider::append_file` projects an accepted `AppendFile` resource into
writer, closer, and syncer capabilities. The resource and all three projected
capabilities retain the registration owner and private slot. Projecting the
resource through `io_provider::read_file`, `io_provider::update_file`, or
`io_provider::duplex` returns `io::ErrorKind::Unsupported`. None of these
failures changes the resource, and a later `append_file` projection still
succeeds.

`io_provider::write_file` projects an accepted `WriteFile` resource into
writer, seeker, positional-writer, closer, syncer, and truncater capabilities.
The resource retains all six captured operation handlers. Each projected
capability retains its corresponding handler plus the registration owner and
private slot after the registration handler scopes exit. Projecting the
resource through `read_file`, `update_file`, `append_file`, or `duplex` returns
`io::ErrorKind::Unsupported`. These failures do not change the resource, and a
later `write_file` projection still succeeds.

`io_provider::update_file` projects an accepted `UpdateFile` resource into
reader, writer, seeker, positional-reader, positional-writer, closer, syncer,
and truncater capabilities in that order. The resource and every capability
retain their registration owner, private slot, and corresponding captured
handler after the supplying handler scopes exit. Projecting the resource
through `read_file`, `write_file`, `append_file`, or `duplex` returns
`io::ErrorKind::Unsupported`. These failures do not change the resource, and a
later `update_file` projection still succeeds.

`io_provider::duplex` projects an accepted `Duplex` resource into a record with
reader, writer, and closer capabilities. The resource and all three
capabilities retain their registration owner, private slot, and corresponding
captured handler after the supplying handler scopes exit.

While the owner remains open, only the matching projection succeeds across all
five families. Every nonmatching projection returns
`io::ErrorKind::Unsupported` without invoking a captured operation handler or
changing the resource. The matching projection still succeeds after each
rejected projection. While the owner remains open, a same-family or
cross-family registration at a used owner slot returns
`io::ErrorKind::InvalidInput` and preserves the first resource and provider
identity regardless of registration order. Owner closure takes precedence and
returns `InvalidResource` as described below.

### Provider-bound dispatch

The exported `fs` namespace facades invoke the `fs::Read` or `fs::Mutate`
handler captured by their filesystem token. The exported `io` byte facades
invoke the operation handler captured by their projected capability after the
request checks described below. A negative `io::read` or `io::read_at` maximum
fails before capability or owner validation and before provider dispatch.
Empty sequential calls take the validated bypass.
Dispatch passes the public token or restricted capability to the handler; it
does not expose the underlying `io_provider::Resource`. A handler can use the
matching slot accessor with its owner while a nested provider scope is active.

One captured handler frame can serve registrations for more than one active
provider owner. Each token, resource, and capability retains its own owner.
Captured dispatch makes only that owner available to matching slot access and
byte-resource registration for the duration of the handler invocation. It
does not grant namespace-token registration authority. Nested captured calls
restore the preceding owner after normal return, a returned failure, or a
runtime failure.

Nested handlers affect newly allocated tokens and capabilities. They do not
reinterpret an older value. A facade call that requires provider invocation
selects the older value's creating handler even after that handler's dynamic
scope exits, provided its provider owner remains usable. Dispatch never selects
the currently active same-effect handler as a fallback. Provider-created
tokens do not perform host filesystem or network fallback. The bounded JVM
system token described below explicitly captures real-host handlers instead.

Direct `io::read` with a zero `ByteCount` validates the reader capability and
that its provider-owner scope remains open, then returns empty `Data` without
invoking the provider. Direct `io::write` with an empty `ByteChunk` performs the
corresponding validation and returns `Written(0)` without invoking the provider.
For every byte-transfer facade call that reaches owner validation, a capability
whose provider-owner scope is closed returns the matching read or write
`InvalidResource` failure with known zero progress, and its provider is not
invoked. This owner rejection does not override the earlier negative-maximum
rejection.

Once the owner is usable, every other accepted byte-transfer request invokes
the selected provider exactly once. This includes `read_at` and `write_at`
requests with a zero maximum or empty chunk. None of the four byte-transfer
facades retries its provider. A negative read maximum returns `ReadFailed` with
empty bytes, known zero progress, and `InvalidInput` through the earlier
pre-validation path.

Each of `io::read`, `io::write`, `io::read_at`, and `io::write_at` validates its
provider reply against the submitted maximum or chunk length. A structurally
valid `Data` success contains from zero through the requested maximum bytes. A
structurally valid `Written` success reports a count in the inclusive range
from zero through the submitted chunk length. A failed read is valid only when
its bytes fit within the request and its known count or unknown lower bound is
between the delivered-byte length and the request bound. A failed write is
valid only when its known count or unknown lower bound is nonnegative and does
not exceed the submitted chunk length. Valid provider failures remain
unchanged, including their original errors and `Known` or `Unknown` progress.

After validation, only sequential positive requests apply no-progress
normalization. Direct `io::read` converts empty provider `Data` to `ReadFailed`
with empty bytes, known zero consumed progress, and `NoProgress`. Direct
`io::write` converts provider `Written(0)` to `WriteFailed` with known zero
committed progress and `NoProgress`. Positional `read_at` preserves empty
`Data`, and positional `write_at` preserves `Written(0)`, including for positive
requests. `End`, nonempty short reads, and positive partial writes remain
unchanged.

An out-of-bounds success or failure reply becomes the operation's
`InvalidResource` failure. A malformed read reply carries empty bytes and
`Unknown(0)` consumed progress. A malformed write reply carries `Unknown(0)`
committed progress. The facade does not wrap or clamp a count, retain the
malformed provider error, or retry the provider. Zero is the only independently
confirmed prefix after a reply contradicts the submitted bounds.

A scoped Veln fake used by the companion tests gives each registered,
already-open resource an independent sequential cursor. Capability aliases
projected from the same resource share that cursor, while a separate resource
over the same bytes does not. Successful
`Start`, `Current`, and `End` seeks set the cursor to the resulting nonnegative
offset. Seeking beyond the current end changes only the cursor; it does not
extend or otherwise change the bytes. A positive-size sequential read there
returns `End` and leaves the cursor at the beyond-end target. For the bounded
in-file transfers in this fake, sequential reads and writes advance by the
delivered or committed count, including partial failures. Positional reads and
writes preserve the sequential cursor on success and failure; a partial
positional write changes only its confirmed prefix.

The fake enters its extension path only when the sequential cursor is strictly
beyond the current end. It defines each byte between the old end and a
committed write there as zero. A successful write extends the bytes through its
reported committed prefix and advances the shared sequential cursor by that
count. A scripted failed write with known zero progress preserves the bytes and
cursor, including when the cursor is beyond the end. A scripted failed write
with known positive progress inserts the required zero gap, commits exactly the
reported input prefix, and advances the cursor by that count.

A sequential write that starts at or before the current end remains a bounded
replacement. If its reported committed prefix would cross the current end, the
fake returns `WriteFailed` with `Known(0)` progress and `InvalidResource`, and
preserves the bytes and cursor. This applies to scripted successes and
failures, including writes that start exactly at the end. Each seek and
nonempty write dispatched to the fake remains present in operation order even
when the write preserves state. This zero-gap rule specifies only the scoped
fake provider; it does not establish a host-filesystem gap policy.

Negative and overflowing seek targets fail without changing the bytes, size,
or prior known cursor. A scripted known-no-change seek failure has the same
preservation rule. A scripted failure can instead make the cursor unknown;
while it is unknown, `Current` and `End` seeks fail, and a successful absolute
`Start` seek restores a known cursor. These observations establish the
stateful test adapter behavior only for this scoped fake. They do not make the
fake a production filesystem implementation, establish host error translation,
or establish the cursor state after a failed host seek.

The JVM `fs::system_handler()` has one bounded real-host path for feasibility
evidence. Within an active provider scope, `fs::system(root)` captures the
selected directory as an absolute rooted token. `fs::open_write` accepts only
an existing portable child path with `WriteMode(Existing, false)`. The returned
`WriteFile` supports sequential write, seek, and close; its positional write,
sync, truncate, other open modes, and other namespace operations are outside
this path. The root check is lexical and does not provide filesystem
confinement: a followed symbolic link in the selected child path can resolve
outside the selected directory.

Publication of the rooted namespace token is atomic with owner closure and
selection of an unoccupied provider slot. If another provider registration
occupies the next host candidate, namespace or `WriteFile` publication
preserves that identity and selects the next free slot. `fs::system(root)`
that reaches publication after owner closure returns `InvalidResource`; it
does not create or replace the closed scope's owner or change an existing
registry. The host-backed `WriteFile` is also rejected when its owner has
closed. If the owner closes after the host channel opens but before
publication, the exact candidate resource is not registered, every preceding
registry identity is preserved, and that channel is closed once. Scope exit
closes each successfully published host file through its captured close
handler.

For that host file, a valid absolute seek beyond end changes the sequential
cursor without extending the file. A later successful sequential write starts
at that cursor, extends the file through the committed suffix, and advances the
cursor by the committed count. Bytes before the old end and the written suffix
remain observable, but bytes in the intervening gap follow the JVM host
contract and have no portable value requirement. A negative or arithmetically
overflowing target is rejected as `InvalidInput` before changing the host
position, so the known cursor, file bytes, and length are preserved. This
preservation rule does not cover an arbitrary host exception after a seek is
attempted; uncertain host cursor failures remain unimplemented.

Ordinary `perform` retains lexical handler selection and does not receive the
broader captured invocation authority. An accepted lexical provider open
receives only a per-call admission identity for publication. It cannot use
that identity to register namespace tokens. After owner closure, a facade or
lexical byte operation nested in the admitted open does not inherit its
publication admission. Registration attempted by the byte handler returns
`InvalidResource` and does not become an admitted late resource. If a direct
low-level operation supplies a filesystem token or byte capability from a
different provider owner, the runtime returns the operation's
`InvalidResource` failure shape before invoking either provider. This applies
when the owners share a handler frame and when different frames captured the
operand. Wrong-owner resource and capability slot access also returns
`InvalidResource`. These failures do not change either provider's `State<T>`.

Owner closure applies the same rejection to facade calls that reach owner
validation and before ordinary lexical `perform` dispatch. Each such operation
returns its documented `InvalidResource` result shape, and the captured or
lexically selected provider is not invoked.

`io::copy` requires a positive chunk size. A non-positive chunk size returns
`InvalidInput` with known zero consumed and committed progress before either
provider is invoked. For a positive size, it reads until `End`. A short read is
data, not end-of-input. The source and destination can belong to different
provider owners.

Each nonempty read chunk is consumed input. `copy` writes the chunk completely
before it reads again. If a write reports a partial success, the next write
receives only the uncommitted suffix. An empty successful read or a zero-byte
successful write returns `NoProgress` without retrying. Successful completion
returns the total committed byte count.

A failed read contributes its reported `Known` or `Unknown` progress to prior
consumed input and leaves prior committed output unchanged. Bytes carried by a
failed read are not written. A failed write reports the complete current read
chunk as consumed and adds its `Known` or `Unknown` committed progress to prior
output progress. Unknown progress remains unknown and stops the transfer.

A read result cannot exceed the requested chunk size. Failed-read bytes cannot
exceed its progress lower bound, and that lower bound must be nonnegative and
cannot exceed the request. A write count or write-failure lower bound must be
nonnegative and cannot exceed the supplied suffix. A provider reply that
violates these bounds, or that would overflow an accumulated count, returns
`InvalidResource`. The affected progress becomes `Unknown` with the last
independently confirmed prefix; the other progress remains exact. The invalid
reply is not used to fabricate a count or another suffix.

`io::read_exact` accepts requested counts from zero through `2147483647`, the
largest count representable by its returned `ByteChunk`. A count outside that
range returns `InvalidInput` with empty bytes and known zero progress before
provider dispatch. A zero count first validates that the reader and its
provider owner remain usable. A valid reader returns empty `Complete` without
a read; an invalid reader returns `InvalidResource` with empty bytes and known
zero progress.

For a positive count, `read_exact` requests only the unfilled remainder. It
accumulates short `Data` results until it has the requested bytes. `End`
returns `Short` with every accumulated byte. An empty `Data` result returns
`NoProgress` with the accumulated bytes and known consumed count. A
`ReadFailed` result returns `ExactFailed` with the bytes accumulated before the
call followed by the failure-delivered bytes. Its consumed progress adds the
previously delivered count to the provider's `Known` or `Unknown` lower bound,
and the helper does not retry the failure.

`io::write_all` first validates an empty write's writer and provider owner. A
valid writer returns `Written(0)` without a write; an invalid writer returns
`InvalidResource` with known zero progress. For a nonempty chunk, each partial
`Written` result causes the next provider call to receive only the uncommitted
suffix. Completion returns the total committed count. A zero count from a
nonempty provider write returns `NoProgress` with the known committed prefix.
A `WriteFailed` result adds its `Known` or `Unknown` lower bound to the
previously committed count and stops without retry.

For both helpers, provider counts must be nonnegative and remain within the
submitted remainder or suffix. Failure-delivered read bytes cannot exceed the
failure progress lower bound. A reply outside those bounds, or a count whose
accumulation would overflow, returns `InvalidResource`. The affected progress
becomes `Unknown` with the last independently confirmed prefix. The helper
does not use the malformed reply to append bytes, invent progress, select a
suffix, or make another provider call.

Two independently registered writers owned by one active provider can share
one provider-state cell. When child tasks reach a readiness gate and are then
released to write distinct chunks, each completed call reports the byte count
of its own chunk. The seeded output retains both whole chunks in either
serialized order. The operation log contains exactly the same two writes in
that order, and unrelated source and read state remain unchanged. This
observation establishes that completed provider transitions are not lost or
partially interleaved under concurrent callers. The gate does not establish
simultaneous runtime admission or a scheduler order.

### Owner closure and restricted release

Owner exit first makes byte-resource projection, facade and lexical capability
dispatch, ordinary slot access, and new publication unavailable. These
operations return their `InvalidResource` failure shape without invoking a
captured or selected operation handler. Ordinary dispatch remains unavailable
during cleanup. This includes recursive `io::close`, which returns the closed
owner failure without re-entering the close handler. The transition applies on
normal return, a returned `Err`, an abrupt contract or modeled runtime failure,
and task cancellation. When restricted cleanup succeeds during an abrupt
failure, the original failure remains primary and execution does not continue
after the handled scope.

The runtime then invokes each registered resource's captured close handler.
During that invocation, the matching owner can inspect the releasing resource
slot and capability slots projected from that exact resource. A stale resource
or capability alias receives `io::ErrorKind::InvalidResource` even when it has
the same owner and numeric slot. A foreign owner receives the same error. The
temporary authority does not permit a new resource registration. Failed access
and failed registration leave unrelated provider `State<T>` cells unchanged.
The close handler's scripted success or failure does not replace an ordinary
body `Result`; the complete F5 primary and related cleanup-failure projection
remains unimplemented.

A failed close outcome or modeled Veln runtime failure is retained at this
bounded owner boundary. A fatal JVM `Error` is not a retained release result.
After the runtime restores the current release authority and removes the
current registered resource and scope frame, the same fatal error escapes to
the JVM execution boundary.

If cancellation is first delivered at a checkpoint in a close handler, the
runtime retains cancellation as the primary unwind. It still attempts all
already-selected release handlers. Execution after the handled owner scope
does not continue, and unrelated provider state remains unchanged.
Cancellation combined with a later ordinary cleanup failure, retry, release
order, and diagnostic projection remain part of F5.

A facade or accepted lexical namespace call that began while the owner was
open can finish its provider call after owner closure. If that provider call
returns a registration candidate, the resource is rejected with
`io::ErrorKind::InvalidResource`, is not inserted into the owner registry, and
is passed once to its captured close handler. A failed close outcome or a
modeled Veln runtime failure from that release is retained by the owner
boundary. This admission decision is atomic with owner closure and
existing-slot validation. An active release for one owner does not change the
release obligation of another owner's independently admitted late resource.
The runtime leaves registry admission before it invokes the close handler, so
an inherited child can reach its own closed-owner registration rejection while
the handler waits for it. This bounded retention is backend-observable;
source-visible cleanup-failure precedence and retry remain part of F5.

## Runtime limits

This surface is not a production filesystem implementation. Runtime
state, namespace-token allocation, registration, projection, provider-bound
dispatch, bounded `copy` outcomes and progress, complete-transfer helper
outcomes, direct facade provider-count validation, serialized completed
fake-provider writes, and the bounded owner-release lifecycle are limited to
the observations above. Remaining provider positional conformance, host
positional conformance, host translation, uncertain host-seek cursor behavior,
the remaining open-mode, namespace, metadata, mutation, truncate, append, and
sync outcomes, cleanup retry and failure precedence, general host filesystem
selection, and network-to-byte adapters remain unimplemented.
Type-checking behavior outside the implemented slice establishes only its
static capability and effect boundary.

## References

- Complete accepted static surface:
  `examples/specification/check/filesystem-io-static-api/`.
- Bounded copy behavior with short reads and suffix-only partial-write retry,
  including the complete outcome and progress matrix:
  [`io_provider.test.veln`](../../crates/veln-stdlib/veln/io_provider.test.veln),
  with scripts and progress logs in
  [`io_copy_test_support_test.veln`](../../crates/veln-stdlib/veln/io_copy_test_support_test.veln).
- Complete read and write behavior, including supported caller-count bounds,
  zero-size validation, partial progress, provider failures, malformed replies,
  and overflow, plus direct empty-request bypass, closed-provider-owner
  rejection, no-progress normalization, all four direct-facade provider-reply
  bounds, and single dispatch:
  [`io.test.veln`](../../crates/veln-stdlib/veln/io.test.veln), with scoped
  scripted providers in
  [`io_complete_transfer_test_support_test.veln`](../../crates/veln-stdlib/veln/io_complete_transfer_test_support_test.veln)
  and
  [`io_direct_validation_test_support_test.veln`](../../crates/veln-stdlib/veln/io_direct_validation_test_support_test.veln).
- Scoped-fake per-open cursor identity, seek targets, sequential progress,
  zero-gap extension after a beyond-end seek, bounded EOF and crossing-EOF
  replacement rejection, failed-write commit state and operation order, and
  positional cursor preservation:
  [`io_provider.test.veln`](../../crates/veln-stdlib/veln/io_provider.test.veln),
  with state and scripted failures in
  [`io_seek_cursor_test_support_test.veln`](../../crates/veln-stdlib/veln/io_seek_cursor_test_support_test.veln).
- Bounded JVM rooted-host writable open, beyond-end seek and sequential
  extension, host-defined gap treatment, and prevalidated failed-seek cursor
  preservation:
  [`filesystem_host_seek_beyond_end.veln`](../../crates/veln-backend-jvm/test-support/filesystem_host_seek_beyond_end.veln),
  exercised by the focused runtime integration test in
  [`runtime_integration.rs`](../../crates/veln-backend-jvm/src/tests/runtime_integration.rs).
- Rooted-host namespace publication under owner closure, namespace and
  write-resource publication under provider-slot contention, rejection after
  owner closure with preservation of preceding registry identities and one
  release of the exact opened channel, and release of the successfully
  published host channel:
  [`HostFilesystemPublicationHarness.java`](../../crates/veln-backend-jvm/test-support/HostFilesystemPublicationHarness.java),
  exercised by the focused runtime integration test in
  [`runtime_integration.rs`](../../crates/veln-backend-jvm/src/tests/runtime_integration.rs).
- Wrong capability operands:
  `examples/specification/check/filesystem-io-capability-boundaries/`.
- Missing read-file closer, write-file truncater, update-file positional
  writer, and append-file syncer:
  `examples/specification/check/filesystem-io-missing-capabilities/`,
  `examples/specification/check/filesystem-io-write-file-missing-truncater/`,
  `examples/specification/check/filesystem-io-update-file-missing-writer-at/`,
  and `examples/specification/check/filesystem-io-append-file-missing-syncer/`.
- Filesystem, file-view projection, and opaque identity boundaries:
  `examples/specification/check/filesystem-io-filesystem-boundaries/` and
  `examples/specification/check/filesystem-io-opaque-identities/`.
- Private compiler-adapter boundary:
  `examples/specification/check/filesystem-io-private-adapters/`.
- Atomic state nominal boundary:
  `examples/specification/check/filesystem-io-atomic-state-nominal-boundary/`,
  including exact-companion constructor and pattern rejection in
  `examples/specification/check/filesystem-io-opaque-state-companion-boundary/`
  and `examples/specification/check/filesystem-io-opaque-state-pattern-boundary/`.
- Atomic state value and expected-type inference, including a nullary generic
  initializer, list and control-flow joins in either contribution order,
  record-width joins, concrete type retention, and foreign-package namesake
  isolation through checked core and typed IR:
  [`io_provider.rs`](../../crates/veln-sema/src/tests/standard_library_effects/lowering_and_types/io_provider.rs).
- Atomic state source-ADT variance joins, invariant-join rejection,
  handler-clause handoff, bounded adjacent-size type analysis, and additive
  import and pattern work when an unavailable state constructor is written
  through an explicit standard-module alias:
  [`bounded_types.rs`](../../crates/veln-sema/src/tests/standard_library_effects/lowering_and_types/io_provider/bounded_types.rs).
- Atomic state generic mismatch, invariant-cell, and pure-callback rejection:
  [`lowering_and_types.rs`](../../crates/veln-sema/src/tests/standard_library_effects/lowering_and_types.rs).
- Atomic allocation identity, shared child identity, serialized completed
  updates, publication, namespace-token handler capture, missing-handler slot
  preservation, duplicate and wrong-owner rejection, closed-owner slot
  rejection, post-close read-only projection, and five-family registration and
  projection behavior, two independent fake providers, shared handler-frame
  registration by two owners, nested captured dispatch, foreign-identity state
  preservation, rejection of namespace registration for a non-nearest owner
  without consuming its slot, the bounded cross-provider copy outcome matrix,
  and two
  same-owner writers sharing one fake-provider state cell without losing or
  interleaving completed chunks:
  [`io_provider.test.veln`](../../crates/veln-stdlib/veln/io_provider.test.veln),
  with pure state transitions in
  [`io_provider_state_test_support_test.veln`](../../crates/veln-stdlib/veln/io_provider_state_test_support_test.veln)
  and scoped registration fakes in
  [`io_provider_test_support_test.veln`](../../crates/veln-stdlib/veln/io_provider_test_support_test.veln).
  The provider-dispatch state and handlers are separate test support in
  [`filesystem_provider_test_support_test.veln`](../../crates/veln-stdlib/veln/filesystem_provider_test_support_test.veln).
  Closed-owner projection, facade and lexical dispatch rejection without
  provider calls, owner-exit ordering, exact-resource slot access,
  foreign-owner and cleanup-time registration rejection, cancellation cleanup,
  cancellation first delivered during release with all selected handlers
  attempted, post-scope non-continuation, and unrelated state preservation are
  checked in `io_provider.test.veln`, with separate scoped support in
  [`io_provider_lifecycle_test_support_test.veln`](../../crates/veln-stdlib/veln/io_provider_lifecycle_test_support_test.veln).
- Private JVM adapter linkage:
  [`java_helpers_and_mapping.rs`](../../crates/veln-backend-jvm/src/tests/java_helpers_and_mapping.rs).
- JVM namespace-token captured-handler identity, post-close read-only
  projection, owner matching, and inherited closed-scope factory and slot
  rejection:
  [`IoProviderFilesystemHarness.java`](../../crates/veln-backend-jvm/test-support/IoProviderFilesystemHarness.java).
- JVM namespace-token and projected-capability dispatch through retained and
  shared handler identities, invocation-owner restoration after normal and
  failed nested calls, restricted operand forwarding, foreign-identity
  rejection before either provider runs, lexical namespace-registration
  authority, closed-owner resource-publication rejection, and absence of
  active host fallback:
  [`IoProviderDispatchHarness.java`](../../crates/veln-backend-jvm/test-support/IoProviderDispatchHarness.java).
- JVM owner closure, gated facade and lexical late publication, successful and
  failed late release, cross-owner admission during cleanup, retained release
  failure, facade and lexical ordinary-use rejection, recursive-close
  non-reentry, exact-resource identity, stale same-slot rejection,
  occupied-slot admission, rejection of publication attempted from a nested
  facade or lexical byte handler, registered release after normal and abrupt
  runtime exits, and fatal JVM release-failure propagation with resource
  removal plus authority and handler-stack restoration:
  [`IoProviderOwnerLifecycleHarness.java`](../../crates/veln-backend-jvm/test-support/IoProviderOwnerLifecycleHarness.java).
- JVM-controlled single-successor admission handoff, cancelled-waiter removal,
  cancellation checkpoints, publication, queued failure release, failure
  identity, reentrancy, task freezing, and reusable 100,000-element list state,
  context, replacement, and result values without nesting-proportional JVM
  stack use:
  [`AtomicStateHarness.java`](../../crates/veln-backend-jvm/test-support/AtomicStateHarness.java).
- JVM retention of the captured operation-handler identities after their
  dynamic scopes exit:
  [`IoProviderAppendFileHarness.java`](../../crates/veln-backend-jvm/test-support/IoProviderAppendFileHarness.java)
  and
  [`IoProviderWriteFileHarness.java`](../../crates/veln-backend-jvm/test-support/IoProviderWriteFileHarness.java).
- JVM validation before `UpdateFile` publication and retention of all eight
  captured handlers:
  [`IoProviderUpdateFileHarness.java`](../../crates/veln-backend-jvm/test-support/IoProviderUpdateFileHarness.java).
- JVM validation before `Duplex` publication and retention of its three
  captured handlers:
  [`IoProviderDuplexHarness.java`](../../crates/veln-backend-jvm/test-support/IoProviderDuplexHarness.java).
- JVM registry identity preservation after same-family and cross-family
  duplicate registration:
  [`IoProviderWriteFileHarness.java`](../../crates/veln-backend-jvm/test-support/IoProviderWriteFileHarness.java)
  and
  [`IoProviderUpdateFileHarness.java`](../../crates/veln-backend-jvm/test-support/IoProviderUpdateFileHarness.java).
