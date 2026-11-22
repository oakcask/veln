---
role: specification
authority: normative
specification-coverage: usage=#imports-and-static-surface; behavior=#capability-boundaries; limits=#runtime-limit
update-when: The exported io, fs, or io_provider static source API, opaque capability identities, file-view declarations, or filesystem IO runtime readiness changes.
---

# Standard-library filesystem and byte IO

The standard package exports `io`, `fs`, and `io_provider`. Their current
contract is a compiler-checked static prototype. It lets provider and generic
IO source declare capability-correct signatures before resource allocation,
captured dispatch, or host filesystem execution exists.

## Imports and static surface

Import the modules explicitly:

```veln
use fs from "std"
use io from "std"
use io_provider from "std"
```

The complete checked surface is exercised by the specification cases listed
under [References](#references).
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
in their declared effect rows. Their result types preserve the proposed data,
progress, and error alternatives through `ReadOutcome`, `WriteOutcome`,
`ExactOutcome`, and `CopyOutcome`.

`fs::Read` declares `open_read`, `metadata`, and `entries` over
`fs::Filesystem`. `fs::Mutate` declares writable opens, directory creation,
removal, rename, and truncation over `fs::MutableFilesystem` or
`fs::Truncater`. Portable names use `path::Path`. Host selection uses
`fs::system(Path)` with `fs::Host`; `fs::system_handler()` discharges that
nominal effect and retains coarse `fs` plus `concurrency`.

The file-view declarations carry these capability fields:

| View | Capabilities |
| --- | --- |
| `ReadFile` | reader, seeker, positional reader, closer |
| `WriteFile` | writer, seeker, positional writer, closer, syncer, truncater |
| `UpdateFile` | reader, writer, seeker, both positional views, closer, syncer, truncater |
| `AppendFile` | writer, closer, syncer |

They are single-variant public ADTs because Veln has no named structural-record
alias syntax. Their constructor payload names preserve the file-view field
contract. Possessing those records does not create or upgrade any opaque
capability value.

`io_provider` exposes opaque `Owner` and `Resource` identities, a nominal
`Scope` effect and `scope()` handler, distinct read-only and mutable filesystem
factories, a `ViewFamily` registration selector, file-view and duplex
projections, and owner-checked slot accessor signatures for every token and
capability family.

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

## Runtime limit

This surface is not a production filesystem implementation. The compiler
adapters currently make the declarations checkable; they do not provide JVM
runtime methods. Resource allocation, captured provider dispatch, owner
lifecycle, handler-family registration, incompatible-family failure, atomic
provider state, transfer behavior, host filesystem access, mutation, and
cleanup remain unimplemented. A source file that type-checks against these
declarations has established only its static capability and effect boundary.

## References

- Complete accepted surface:
  `examples/specification/check/filesystem-io-static-api/`.
- Wrong and missing capability operands:
  `examples/specification/check/filesystem-io-capability-boundaries/` and
  `examples/specification/check/filesystem-io-missing-capabilities/`.
- Filesystem, file-view projection, and opaque identity boundaries:
  `examples/specification/check/filesystem-io-filesystem-boundaries/` and
  `examples/specification/check/filesystem-io-opaque-identities/`.
