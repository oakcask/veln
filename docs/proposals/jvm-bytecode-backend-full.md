# JVM Bytecode Backend Full

Status: proposed

This page expands [jvm-bytecode-backend.md](jvm-bytecode-backend.md). It is
planned behavior, not current specification behavior.

## Route Map

- Current proposal status and next implementation work:
  [implementation status](#implementation-status).
- Current completion gate review:
  [../reviews/jvm-bytecode-backend-completion.md](../reviews/jvm-bytecode-backend-completion.md).
- Current implemented behavior:
  [../specification/execution.md](../specification/execution.md) and
  [../specification/commands.md](../specification/commands.md).
- Harness organization:
  [../reference/toolchain-test-harness.md](../reference/toolchain-test-harness.md).
- Promotion mechanics:
  [implementation-route.md](implementation-route.md).

## Implementation Status

This proposal is not complete.

Current behavior already removes the separate `javac` process from ordinary
`run` and `test` execution. It also keeps missing `java` as the user-facing
runner setup error, uses a persistent JVM class cache, and keys that cache with
the compiler helper contents.

That is a migration step, not the accepted backend route. The backend still
exposes `JavaProgram` and generated Java source as the lowering output. The CLI
writes a JVM-hosted compiler helper and launches `java` on that helper, which
then uses the host Java Compiler API. The updated tests prove that a `javac`
executable is not required; they do not prove that source-level Java generation
is gone.

Remaining proposal work:

- add a Rust classfile writer behind the backend API so typed IR lowers
  directly to class files
- keep Java source lowering only as a migration baseline while a test-harness
  backend selector compares it with true classfile lowering
- add parity fixtures for the implemented IR subset, then keep those fixtures
  as bytecode runtime regressions after the Java source route is removed
- add `javap -verbose` smoke tests for stable structural facts
- add the required JVM backend CI job after structural checks exist
- promote only observable setup, runtime, and command JSON behavior into the
  specification after direct classfile emission is implemented

The remaining work is not satisfied by moving the existing Java source
compilation helper behind a different API. A completion claim must remove
source-level Java generation from the ordinary `run` and `test` backend path.
Current gate evidence lives in
[../reviews/jvm-bytecode-backend-completion.md](../reviews/jvm-bytecode-backend-completion.md).

## Problem

The implemented JVM backend lowers typed IR to generated Java source, invokes
`javac`, and then runs the resulting classes. That route was useful for the
first executable slice because it kept the backend simple while the typed IR
was still settling.

The reference execution boundary is already typed IR, not Java source. Keeping
Java source generation as the long-term backend path adds a tool dependency,
couples runtime execution to Java source emission, and makes some backend
regressions visible only after a second compiler has accepted generated code.

## Decision

Change the JVM backend to emit JVM class files directly from executable typed
IR.

The bytecode backend should preserve the existing observable behavior of
`veln run` and `veln test`. It may change generated artifacts, cache contents,
backend helper classes, and setup requirements, but those changes are backend
implementation details unless the language specification later marks them
observable.

During migration, the Java source backend and bytecode backend may coexist
behind an explicit internal selector so parity tests can compare them. After
the bytecode backend becomes the only JVM lowering path, the parity fixtures
remain as bytecode runtime regression tests.

## Non-Goals

- Do not make JVM class names, descriptors, helper names, stack layout,
  constant-pool layout, local variable slots, or instruction sequences part of
  the Veln language contract.
- Do not introduce a stable Java interop surface.
- Do not expose bytecode generation as a user-facing compiler phase unless a
  later proposal accepts that workflow.
- Do not use this proposal to change typed IR semantics, command JSON shape,
  stdio ordering, contract blame, value freezing, task behavior, or channel
  behavior.
- Do not require source-level Java generation to remain available after the
  migration is complete.

## Runtime Behavior Harness

Add a backend-matrix runtime harness for executable fixtures. Each fixture
should declare:

- the command, such as `run` or `test`
- source fixtures and project files
- selected entry, test filter, arguments, and environment overrides when needed
- expected exit status
- expected stdout and stderr fragments
- expected JSON paths or fragments for machine-readable modes
- required host tools such as `java`, `javac`, or `javap`
- backend modes to run, such as Java source, bytecode, or parity comparison

During migration, parity comparison should run the same fixture through each
available JVM lowering path and compare normalized observable output: exit
status, stdout, stderr, structured JSON records, test events, and runtime
contract failures. Backend setup diagnostics should be tested separately
because direct classfile emission intentionally changes the `javac` dependency.
The harness must not compare generated Java source, classfile bytes, bytecode
instruction sequences, constant-pool indexes, class names, local variable
slots, or helper layout.

After migration, the same fixture layout should continue to exercise the
bytecode backend directly.

## Fixture Scope

Runtime fixture coverage should include:

- basic execution: literals, locals, calls, tail returns, omitted tail
  expressions, integer operators, boolean operators, records, vecs, and field
  access
- control flow: `match` over literals, `_`, bindings, `Option`, and `Result`
  constructors, plus `?` early returns
- runtime values: frozen records, vecs, dictionaries, updates that do not
  mutate inputs, `Some`, `None`, `Ok`, and `Err`
- contracts: `require` caller blame, `ensure` implementation blame, contract
  failures in human output, contract failures in JSON output, and `ensure`
  checks before `?` early returns
- stdio and test events: stdout forwarding, stderr forwarding,
  `test --json` event ordering, mixed stdout and stderr sequence stability,
  and task stdio serialization at the runtime handler boundary
- standard library intrinsics: file-system operations, current-process
  operations, argument passing, environment lookup, working-directory lookup,
  and process exit behavior
- concurrency: bounded channels, zero-capacity rendezvous channels, close
  behavior, selection behavior, task spawn, join, and cancellation
- runner setup: no `javac` requirement for bytecode execution, clear missing
  `java` errors, and unchanged command-visible behavior across cache hits and
  misses

The harness should prefer semantic JSON assertions and stream fragments over
full-output equality unless exact envelope shape is the behavior under test.

## Bytecode Verification Coverage

Generated class files should be tested at two levels.

First, runtime fixtures must load and execute generated classes with `java`.
This is the authoritative JVM verifier boundary: class files rejected by the
host JVM fail before observable Veln behavior can be produced.

Second, bytecode-specific tests should run `javap -verbose` on generated
classes and check stable structural facts:

- each generated class can be disassembled
- the selected classfile version matches the backend target
- entry wrappers have the expected JVM descriptors
- generated methods include verification metadata required by the selected
  classfile target
- optional debug attributes are present only when the backend promises them
- ordinary bytecode execution does not require `javac`

`javap` output is diagnostic evidence, not a language contract. Tests should
match normalized fragments rather than complete disassembly output. They must
avoid depending on constant-pool indexes, bytecode offsets, exact local slot
allocation, complete instruction sequences, and backend-private helper names
unless a specific backend invariant intentionally fixes them.

## CI Strategy

CI should be layered so the bytecode backend is covered without making every
pull request run the slowest matrix.

Required pull request checks should include:

- normal Rust workspace tests
- bytecode backend unit tests
- JVM runtime fixture tests on one pinned JDK
- `javap` structural tests for generated classes
- setup behavior tests proving `javac` is not required by bytecode execution
  and missing `java` remains a runner error

Optional or scheduled checks should include:

- supported operating-system matrix coverage
- multiple supported JDK lines
- larger concurrency stress fixtures
- full Java-source versus bytecode parity coverage after the basic parity suite
  is stable

The default Rust test job should continue to cover parsing, checking, typed IR
lowering, backend construction, and non-JVM setup behavior. A dedicated JVM
backend CI job should install the required JDK tools and run runtime behavior
fixtures plus bytecode structural checks.

Missing `javap` should affect only structural bytecode tests. It must not make
ordinary `run` or `test` execution depend on `javap`.

## Cache And Setup Behavior

The backend cache should move from generated Java source and compiled classes
to bytecode backend artifacts. Cache keys may include backend version,
classfile target, runtime helper version, typed IR content, entry selection,
and relevant backend options.

Cache hits and misses must preserve command-visible behavior: exit status,
stdout, stderr, runtime contract reports, and test events must be the same as
if the selected program was generated for the current invocation.

The bytecode backend should remove `javac` from ordinary execution setup.
Missing `java` remains a runner error because the selected program still runs
on the JVM. Missing `javap` is a test-environment issue for structural tests,
not a user-facing runtime requirement.

## Completion Criteria

Use this section to decide whether an implementation claim is complete enough
to promote into the specification.

The implemented IR subset is every typed IR construct that the current JVM
backend accepts on the ordinary `run` and `test` paths. It includes all runtime
fixture groups listed under [fixture scope](#fixture-scope). An implementation
that excludes any group is incomplete unless this proposal is revised to name
the excluded construct and explain why the current JVM backend should stop
accepting it.

A completion claim must satisfy all of these gates:

- Direct classfile emission is the ordinary backend path: `run` and `test`
  lower typed IR to classfile artifacts for the selected program and required
  helpers without writing generated Java source or invoking a Java source
  compiler.
- Existing JVM runtime behavior fixtures pass through the bytecode path, and
  cache hits and misses preserve command-visible exit status, stdout, stderr,
  runtime contract reports, and test events.
- Missing `java` remains a runner error with the existing human and JSON
  behavior, while setup coverage proves `javac` is no longer required for
  ordinary execution.
- While the Java source backend still exists, the CLI fixture harness has an
  internal migration matrix with bytecode, Java-source, and parity modes. The
  selector must not become a public CLI flag, stable JSON field, or
  user-facing diagnostic.
- Bytecode-specific tests verify stable structural facts through class loading
  and `javap -verbose`: classfile target version, expected entry descriptors,
  required verifier metadata, and absence of an ordinary `javac` requirement.
- No test treats classfile bytes, complete `javap` output, constant-pool
  indexes, bytecode offsets, local variable slots, helper names, or ordinary
  instruction ordering as stable language facts.
- A checked-in JVM backend workflow runs bytecode backend unit tests, runtime
  fixture tests, setup tests, and structural bytecode tests on one pinned JDK.

## Implementation Defaults

These answers guide the initial implementation unless later dependency review
or proposal revision changes the constraints.

- Prefer `ristretto_classfile` for the first bytecode writer spike because it
  supports classfile reading, writing, and verification. Use the newest release
  after dependency review. If that release requires a newer Rust toolchain,
  update the repository Rust toolchain policy as part of the same backend
  adoption work instead of selecting an older crate release only to avoid the
  toolchain update.
- Target Java 8 class files for the first bytecode backend. This keeps generated
  classes runnable on newer JDK lines while matching the current runtime helper
  surface. The backend should generate verifier metadata required by that
  target, including `StackMapTable` entries for generated methods with control
  flow.
- Emit only required verifier and execution metadata at first. Optional
  source-span debug attributes such as `SourceFile` and `LineNumberTable` may be
  added later behind an internal debug option. Do not emit `LocalVariableTable`
  as an initial promise because it can accidentally freeze local-slot layout.
- Keep the Java source backend only as a migration-time parity and debug path
  after bytecode becomes the default. Do not expose it as a stable user-facing
  fallback. Remove it once bytecode runtime fixtures and structural checks cover
  the implemented IR subset.
- Expose backend selection through the test harness, not through public CLI
  syntax. Fixture manifests may name internal backend modes such as Java source,
  bytecode, or parity. If command-level selection is needed for integration
  tests, use a clearly internal test-only environment variable and keep it out of
  the language specification.
- Pin the required JVM backend CI job to an OpenJDK JDK distribution that can run
  Java 8 class files and provides `javap -verbose`. The job should install a JDK,
  not only a JRE, because structural classfile smoke tests depend on JDK tools.

The remaining implementation-time checks are:

- Confirm the selected `ristretto_classfile` release, transitive dependency
  surface, unsafe-code policy, license metadata, and required Rust toolchain
  before adding it to the workspace.
- Confirm the harness selector cannot become observable command behavior through
  documented flags, stable JSON fields, or user-facing diagnostics.

## Promotion Route

When implementing this proposal, compare it against current behavior in
[../specification/execution.md](../specification/execution.md),
[../specification/commands.md](../specification/commands.md), and
[../reference/toolchain-test-harness.md](../reference/toolchain-test-harness.md).
After implementation, promote only observable command and runtime behavior into
the specification pages. Keep generated artifacts, bytecode layout, helper
layout, backend selectors, and structural test details out of the language
specification unless a later proposal makes them user-facing behavior.
