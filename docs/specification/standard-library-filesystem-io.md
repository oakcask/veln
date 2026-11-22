---
role: specification
authority: normative
specification-coverage: usage=#imports-and-static-surface; behavior=#file-view-runtime-prototype; limits=#runtime-limits
update-when: The exported io, fs, or io_provider static source API, opaque capability identities, file-view declarations, or filesystem IO runtime readiness changes.
---

# Standard-library filesystem and byte IO

The standard package exports `io`, `fs`, and `io_provider`. Their current
contract combines a compiler-checked static surface with bounded `ReadFile`,
`WriteFile`, and `AppendFile` registration and projection prototypes. It lets provider and
generic IO source check capability-correct signatures and exercise three runtime
resource families.
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
`Busy`, `InvalidResource`, `Cancelled`, `NoProgress`, or `Other`. These are
declarations only; the [runtime limits](#runtime-limits) mean no filesystem or
byte operation currently produces these outcomes at runtime.

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

`io_provider` exposes the adapter surface below. The factories and both slot
groups retain `concurrency`; `owner` requires `Scope`, the `scope` handler
retains `concurrency`, and the resource projections are pure.

| Group | Signatures |
| --- | --- |
| Scope and factories | `owner() -> Owner effects [Scope]`; `scope()` handles `Scope` with `concurrency`; `filesystem(Owner, Int) -> Result<fs::Filesystem, io::Error>`; `mutable_filesystem(Owner, Int) -> Result<fs::MutableFilesystem, io::Error>`; `resource(Owner, Int, ViewFamily) -> Result<Resource, io::Error>` |
| Resource projections | `read_file(Resource) -> Result<fs::ReadFile, io::Error>`; `write_file(Resource) -> Result<fs::WriteFile, io::Error>`; `update_file(Resource) -> Result<fs::UpdateFile, io::Error>`; `append_file(Resource) -> Result<fs::AppendFile, io::Error>`; `duplex(Resource) -> Result<{reader: io::Reader, writer: io::Writer, closer: io::Closer}, io::Error>` |
| Identity slots | `slot(Owner, Resource) -> Result<Int, io::Error>`; `filesystem_slot(Owner, fs::Filesystem) -> Result<Int, io::Error>`; `mutable_filesystem_slot(Owner, fs::MutableFilesystem) -> Result<Int, io::Error>` |
| Capability slots | `reader_slot(Owner, io::Reader)`, `writer_slot(Owner, io::Writer)`, `seeker_slot(Owner, io::Seeker)`, `reader_at_slot(Owner, io::ReaderAt)`, `writer_at_slot(Owner, io::WriterAt)`, `closer_slot(Owner, io::Closer)`, `syncer_slot(Owner, io::Syncer)`, and `truncater_slot(Owner, fs::Truncater)` each return `Result<Int, io::Error>` |

`ViewFamily` is `ReadFile`, `WriteFile`, `UpdateFile`, `AppendFile`, or
`Duplex`. The registration selector does not add capabilities by itself.
Only the bounded `ReadFile`, `WriteFile`, and `AppendFile` runtime behavior described below
is implemented.

The compiler adapters behind these declarations are available only to source
embedded in the standard package. Application source cannot call their
reserved `prelude_builtin` names, including by changing the name casing. Each
admitted adapter has the same fixed operand and result types as its standard
wrapper; an expected type at a call site cannot reinterpret an owner as a byte
capability or replace a required owner operand with an integer.

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

## File-view runtime prototype

Within an active `io_provider::scope`, `io_provider::resource` accepts an
unused nonnegative private slot for `ViewFamily::ReadFile` only when active
Veln handlers provide `io::Read`, `io::Seek`, `io::ReadAt`, and `io::Close`.
It captures those handlers without invoking them. If any handler is absent,
registration returns `io::ErrorKind::Unsupported` before publishing the
resource. This failure performs no host filesystem or network fallback and
does not consume the slot, so a complete registration can subsequently use
the same owner and slot. A negative slot or a slot already registered to that
owner returns `io::ErrorKind::InvalidInput`; a duplicate registration does not
replace the existing resource.

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

`io_provider::read_file` projects an accepted resource into reader, seeker,
positional-reader, and closer capabilities. The resource and every projected
capability retain the registration owner and private slot, as observed by the
matching slot accessors. Projecting the resource through
`io_provider::duplex` returns `io::ErrorKind::Unsupported`. That failed
projection does not alter the resource, and a later `read_file` projection
still succeeds.

`io_provider::append_file` projects an accepted `AppendFile` resource into
writer, closer, and syncer capabilities. The resource and all three projected
capabilities retain the registration owner and private slot. Projecting the
resource through `io_provider::read_file` or `io_provider::duplex` returns
`io::ErrorKind::Unsupported`. Neither failure changes the resource, and a
later `append_file` projection still succeeds.

`io_provider::write_file` projects an accepted `WriteFile` resource into
writer, seeker, positional-writer, closer, syncer, and truncater capabilities.
The resource and all six capabilities retain the registration owner, private
slot, and captured operation handler after the registration handler scopes
exit. Projecting the resource through `read_file`, `append_file`, or `duplex`
returns `io::ErrorKind::Unsupported`. These failures do not change the
resource, and a later `write_file` projection still succeeds.

## Runtime limits

This surface is not a production filesystem implementation. Runtime
registration and projection are limited to the observations above. `UpdateFile`,
`Duplex`, and compatibility combinations not described above remain static
declarations only. Operation dispatch through a resource, multiple-provider
allocation, atomic provider state, publication
and owner-close races, transfer and namespace behavior, cleanup, and host
filesystem access remain unimplemented. Type-checking any other declaration
establishes only its static capability and effect boundary.

## References

- Complete accepted static surface:
  `examples/specification/check/filesystem-io-static-api/`.
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
- `ReadFile`, `WriteFile`, and `AppendFile` registration, missing-handler rejection, slot
  preservation, and projection behavior on the JVM-backed companion runner:
  [`io_provider.test.veln`](../../crates/veln-stdlib/veln/io_provider.test.veln),
  with scoped fakes in
  [`io_provider_test_support_test.veln`](../../crates/veln-stdlib/veln/io_provider_test_support_test.veln).
- Private JVM adapter linkage:
  [`java_helpers_and_mapping.rs`](../../crates/veln-backend-jvm/src/tests/java_helpers_and_mapping.rs).
- JVM retention of the captured operation-handler identities after their
  dynamic scopes exit:
  [`IoProviderAppendFileHarness.java`](../../crates/veln-backend-jvm/test-support/IoProviderAppendFileHarness.java)
  and
  [`IoProviderWriteFileHarness.java`](../../crates/veln-backend-jvm/test-support/IoProviderWriteFileHarness.java).
