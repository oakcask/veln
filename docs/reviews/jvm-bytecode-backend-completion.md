# JVM Bytecode Backend Completion Review

This review records the current completion gate for the selected JVM bytecode
backend proposal. It is review evidence, not current language behavior.

## Read First

- Proposal target: [../proposals/jvm-bytecode-backend.md](../proposals/jvm-bytecode-backend.md).
- Completion gates:
  [../proposals/jvm-bytecode-backend-full.md#completion-criteria](../proposals/jvm-bytecode-backend-full.md#completion-criteria).
- Current implemented behavior:
  [../specification/execution.md](../specification/execution.md) and
  [../specification/commands.md](../specification/commands.md).

## Result

The ordinary `run` and `test` backend path now satisfies the direct classfile
emission gate and has been promoted into the execution and command
specifications.

## Findings

The ordinary command path no longer builds `JavaProgram` artifacts for `run`
or `test`. It lowers typed IR to a `JvmProgram`, writes emitted `.class`
artifacts into the persistent JVM cache on cache miss, and invokes `java` on
the cached `VelnEntry` class. The command path no longer writes generated Java
source, no longer writes a compiler helper, and no longer invokes a Java source
compiler during ordinary execution.

Runtime coverage now exercises the bytecode path through the CLI integration
tests, including cache hits and misses, missing `java`, setup without `javac`,
stdio, contracts, process intrinsics, channels, tasks, function-typed values,
doctest execution, and the comparison example. Backend unit tests also assert
that classfile artifacts are emitted without Java sources, can be loaded and
run with `java`, and expose the expected classfile target version and entry
descriptor through `javap -verbose`.

A dedicated JVM backend workflow runs the bytecode backend tests and CLI
fixture coverage on a pinned JDK. The old Java source backend API remains in
the backend crate as a migration baseline for source-generation tests, but it
is not used by ordinary `run` or `test`.

## Next Handoff

- Keep expanding bytecode-specific structural checks only around stable backend
  facts.
- Remove or hide the Java source backend API once bytecode unit and fixture
  coverage fully replace source-generation tests.
- Add an internal parity selector only if the Java source baseline remains
  needed for migration work.

## Boundaries

Do not use this review to add bytecode behavior to `../specification/`. Promote
only behavior that has been implemented and is observable through current
command or runtime behavior.
