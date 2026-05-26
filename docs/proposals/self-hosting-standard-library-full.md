# Self-Hosting Standard Library Full

Status: promoted
Implementation: implemented subset: descriptor-backed standard symbols,
minimal `fs` and `process` intrinsics, one source-backed pure helper, and the
compiler-support source-loading trial are promoted to the language reference.

Read [self-hosting-standard-library.md](self-hosting-standard-library.md)
first unless you need the complete implementation proposal.

Use this proposal as historical context when adding standard library surface
area needed for an eventual self-hosting compiler. The matching reference pages
describe current behavior.

## Goal

Move ordinary reusable behavior into Veln libraries while keeping the compiler
responsible for only the primitive runtime boundary it must know before
user-defined effects and effect handlers exist.

The target is not to make the first standard library large. The target is to
make each new library symbol representable as data so it can begin as a
compiler-known intrinsic and later move to a Veln implementation without
changing user-facing names, types, effects, or diagnostics.

## Non-Goals

- Do not expose user-defined effect declarations or source-level effect handler
  syntax as part of this proposal.
- Do not add package management, version solving, or third-party dependency
  resolution.
- Do not promise collection complexity guarantees beyond the current language
  reference until runtime representations are chosen.
- Do not replace existing compiler-known `stdio`, concurrency, or prelude
  behavior in one migration.
- Do not add broad formatting, display protocols, byte streams, terminal
  control, subprocess pipelines, or asynchronous file APIs.

## Design Boundary

The compiler may know a standard library symbol for three reasons:

- Type and effect metadata are needed so checking can proceed before the
  symbol has a Veln implementation.
- Lowering needs a host runtime operation, such as file access or process
  arguments.
- Diagnostics need stable provenance for public-boundary effect drift,
  captured output, or runtime failures.

The compiler should not hard-code ordinary library control flow when the same
behavior can be written in Veln using existing language features. For example,
`list_try_map` may remain compiler-known while callbacks and generics are
limited, but the long-term target is a Veln library implementation once
function values, containers, and error propagation are sufficient.

## Symbol Descriptor

Introduce an implementation-owned standard symbol table. Each compiler-known
library entry should be declared in one place with this logical shape:

```text
module: "fs"
name: "read_to_string"
kind: intrinsic
params: [Path]
return: Result(String, FsError)
effects: [fs]
lowering: runtime.fs.read_to_string
```

The concrete Rust representation can differ, but the table should preserve
these fields:

- `module`: qualified namespace visible to Veln source.
- `name`: source-visible function name.
- `kind`: `intrinsic`, `runtime`, or `veln`.
- `params`: checked parameter types.
- `return`: checked return type.
- `effects`: coarse effect labels introduced by the symbol.
- `lowering`: backend or runtime operation for non-Veln implementations.
- `stability`: whether the symbol is required by self-hosting, experimental,
  or compatibility-only.

Existing hard-coded entries for `stdio`, channel and task operations, and
prelude helpers should move behind this descriptor table incrementally. The
first migration may keep old helper functions as adapters over the table.

## Library Layers

### Core

`core` contains names that should be available without importing a module:

- Primitive container and result types already accepted by the checker.
- `Option` and `Result` constructors and helper functions.
- Integer, boolean, string, and unit basics needed by compiler code.

The current prelude helper names may remain in the prelude namespace while the
implementation learns to route them through symbol descriptors.

### Collections

`collections` should cover compiler data structures before broad application
convenience:

- `List`: length, empty check, append, concatenate, map, filter, fold,
  fallible map, indexing, slicing, and sorting only when needed.
- `Dict`: lookup, contains, insert, remove, keys, values, and iteration shape.
- `Set`: contains, insert, remove, union, intersection, and conversion from
  lists.

Initial collection APIs should stay immutable. Operations return new values or
derived values rather than mutating inputs.

### String

`string` should provide the minimum text processing needed by lexer, parser,
diagnostic, and path code:

- length and emptiness checks;
- prefix, suffix, and contains checks;
- split and split-once;
- trim operations;
- integer parsing and integer rendering;
- joining lists of strings.

Formatting protocols are deferred. A small set of explicit conversion helpers
is acceptable before a display design exists.

### IO And FS

`io` and `fs` should start as text-file operations only:

```text
fs::read_to_string(Path) -> Result(String, FsError) effects [fs]
fs::write_string(Path, String) -> Result((), FsError) effects [fs]
fs::exists(Path) -> Result(Bool, FsError) effects [fs]
fs::read_dir(Path) -> Result(List(Path), FsError) effects [fs]
```

`Path` may start as a named wrapper around `String` if the type system cannot
yet express an opaque library type. The source-visible API should still use the
`Path` name so the representation can change later.

Binary files, streaming, permissions, symlinks, metadata, watchers, and
platform-specific path normalization are deferred.

### Process

`process` should provide only the current program environment needed by a
self-hosted compiler driver:

```text
process::args() -> List(String) effects [process]
process::env(String) -> Option(String) effects [process]
process::cwd() -> Result(Path, ProcessError) effects [process]
process::exit(Int) -> () effects [process]
```

Subprocess spawning is deferred. `process::exit` is an effectful terminating
operation and may require a dedicated lowering path before the general effect
handler design exists.

### Compiler Support

`compiler_support` is allowed as an internal standard library layer while the
compiler is moving toward self-hosting. It may contain stable data utilities
that compiler code needs but ordinary users should not treat as the final
public API:

- source spans and source files;
- diagnostic builders;
- stable IDs;
- small immutable maps and sets if the public collection layer is not ready.

Anything in this layer should either graduate into `core`, `collections`,
`string`, `fs`, or `process`, or remain explicitly internal.

## Effects

Until user-defined effects and handlers exist, standard library effects remain
compiler-known coarse labels.

Required labels for this proposal:

- `fs`: file system reads, writes, directory access, and path-dependent
  existence checks.
- `process`: arguments, environment, current working directory, and exit.

`stdio` remains the existing output effect. File operations should not reuse
`stdio`. Process environment access should not be modeled as pure because it
depends on ambient runtime state.

Missing-effect diagnostics should continue to report the failed public
boundary fact as the primary message. The standard library call that introduced
the effect belongs in related context.

## Implementation Plan

### Step One: Descriptor Table

Implementation review: complete for the current slice. Semantic analysis now
has an implementation-owned descriptor table for compiler-known standard
symbols, and the current behavior pages describe the promoted subset. Stdio
effect metadata, concurrency effect metadata, and prelude helper admission flow
through descriptors while existing type adapters and runtime lowering remain in
place.

- Add a standard symbol descriptor type in semantic analysis or a shared core
  crate.
- Register existing `stdio` calls through descriptors.
- Register existing prelude helpers through descriptors without changing
  source-visible behavior.
- Preserve existing diagnostics and tests while removing duplicated
  signature/effect match logic where practical.

Exit criteria:

- Existing `stdio`, prelude, channel, and task behavior still checks and runs.
- At least one test proves effect metadata comes from the descriptor table.
- At least one test proves a prelude helper signature comes from the descriptor
  table.

### Step Two: Minimal FS

Implementation review: complete for the current slice. Semantic analysis now
recognizes the `fs` effect and the accepted minimal `fs` symbols through the
standard symbol table. Type checking, body effect inference, public-boundary
diagnostics, checked-core lowering, typed IR lowering, and JVM runtime lowering
cover the implemented surface. The current runtime represents `Path` with the
string-backed representation described in the language reference. Coverage
includes a JVM entry run that reads a text file through `fs::read_to_string`
and prints it through `stdio`.

- Add the `fs` effect label.
- Add descriptors for `fs::read_to_string`, `fs::write_string`, `fs::exists`,
  and `fs::read_dir`.
- Lower these operations through the active runtime backend.
- Return `Result` instead of throwing host exceptions into Veln execution.
- Add public-boundary missing-effect diagnostics for transitive `fs` calls.

Exit criteria:

- A public function that calls `fs::read_to_string` without `effects [fs]`
  reports an effect diagnostic at the public signature.
- A private helper may call `fs::read_to_string`, and the effect propagates to
  the public caller.
- A runnable program can read a text file and print its contents through
  existing `stdio`.

### Step Three: Minimal Process

Implementation review: complete for the current slice. Semantic analysis now
recognizes the `process` effect and the accepted minimal process symbols
through the standard symbol table. Type checking, body effect inference,
public-boundary diagnostics, checked-core lowering, typed IR lowering, and JVM
runtime lowering cover argument access, environment lookup, current working
directory reporting, and exit. Coverage includes JVM runtime checks for entry
argument capture, missing environment keys returning `None`, and `cwd`
returning `Result`.

- Add the `process` effect label.
- Add descriptors for `process::args`, `process::env`, `process::cwd`, and
  `process::exit`.
- Define runtime behavior for unavailable environment keys and invalid exit
  status values.
- Keep subprocess spawning out of scope.

Exit criteria:

- A public function using process arguments requires `effects [process]`.
- `process::env` returns `None` for a missing key.
- `process::cwd` reports runtime failure as `Result`.

### Step Four: Library-Backed Helpers

Implementation review: complete for the current slice. The compiler build
embeds a Veln source implementation for `option_unwrap_or` and records that
source on the helper's standard symbol descriptor. The checker still uses the
existing descriptor-backed type adapter, and the JVM backend still uses the
existing prelude runtime lowering, so user programs observe the same name,
type, effects, diagnostics, and runtime result as before. Broader migration of
pure helpers to source execution is still pending.

- Identify prelude helpers whose bodies can be expressed in current Veln.
- Add a build or embedding path for standard Veln source files.
- Prefer Veln implementations for pure helpers while preserving the same
  descriptors for type and effect metadata.
- Keep runtime intrinsics only for operations that cannot be expressed in Veln.

Exit criteria:

- At least one pure helper is implemented in Veln source and imported by the
  compiler build.
- User programs observe the same name, type, effects, and runtime result as
  before.
- Diagnostics still point at user call sites rather than standard library
  implementation internals unless the standard library source is explicitly
  being checked.

### Step Five: Compiler Subset Trial

Implementation review: complete for the current slice. The compiler build
embeds a Veln `compiler_support` source-loading helper and the test suite
checks and runs it through the descriptor-backed `fs` subset. The Rust compiler
remains the host driver, and broader self-hosted compiler subsystems remain out
of scope for this proposal.

- Write one small compiler subsystem in Veln using only the accepted standard
  library subset.
- Candidate subsystems are source file loading, token classification, command
  argument normalization, or diagnostic JSON assembly.
- Keep the Rust compiler as the host driver while compiling and running the
  Veln subsystem.

Exit criteria:

- The Veln subsystem is built and exercised by the existing test suite.
- Its dependencies are limited to descriptor-backed standard symbols and Veln
  source helpers.
- Any missing library operation needed by the subsystem is added through this
  proposal's descriptor process rather than as an ad hoc compiler special case.

## Documentation Updates

When a step is implemented, update the smallest current-behavior pages:

- Names, prelude, and effect behavior:
  [../reference/language/names-effects.md](../reference/language/names-effects.md).
- Runtime execution and command behavior:
  [../reference/language/execution.md](../reference/language/execution.md)
  and [../reference/language/commands.md](../reference/language/commands.md)
  when command output changes.
- Types:
  [../reference/language/types.md](../reference/language/types.md) when `Path`,
  `FsError`, `ProcessError`, or collection types become source-visible
  current behavior.
- Diagnostics JSON:
  [../reference/language/diagnostics-json.md](../reference/language/diagnostics-json.md)
  only when machine-readable diagnostic fields change.

Do not promote this whole proposal at once. Promote only the implemented
subset.

## Open Questions

- Should standard library modules require explicit `use` declarations, or can
  first standard modules remain globally qualified like current `stdio`?
- Should `Path` be an opaque type, a record, or a named string wrapper in the
  first source-visible version?
- Should `fs::exists` return `Bool` or `Result(Bool, FsError)` when permission
  errors prevent reliable existence checks?
- Should `process::exit` be typed as returning `()` or a future never type?
- How should standard library source be distributed once package management
  exists?

## Promotion Rule

This proposal has been promoted in slices. New standard library surface remains
promotable only when its descriptors, type checking, effect propagation,
lowering, runtime behavior, tests, and reference documentation are all present
for the selected symbols.
