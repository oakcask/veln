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

The proposal is not complete and must not be promoted into the specification
yet.

## Findings

The ordinary command path still fails the direct classfile emission gate.
`run` builds a `JavaProgram` through the Java backend API, and the JVM command
helper writes each generated Java source file into the class cache preparation
directory. It also writes a JVM-hosted compiler helper and launches `java` on
that helper. The helper uses the Java Compiler API to compile those generated
sources into class files. That removes a separate `javac` executable from
ordinary setup, but it does not make typed IR lower directly to classfile
artifacts.

The testing and CI gates are also incomplete. Existing backend tests compile
generated Java sources when a Java compiler is available. The checked-in
workflows run the normal Rust workspace tests, but there is no dedicated JVM
backend workflow that runs bytecode runtime fixtures, setup tests, and
`javap -verbose` structural checks on a pinned JDK. There is also no internal
test-harness selector with bytecode, Java-source, and parity modes for
normalizing observations across both lowering paths.

## Next Handoff

- Add a backend API that returns classfile artifacts rather than generated Java
  source.
- Make ordinary `run` and `test` use that bytecode path by default.
- Keep the Java source path only as an internal migration baseline while parity
  fixtures compare observable command behavior.
- Add stable `javap -verbose` structural checks and the dedicated JVM backend
  workflow before making a completion claim.

## Boundaries

Do not use this review to add bytecode behavior to `../specification/`. Promote
only behavior that has been implemented and is observable through current
command or runtime behavior.
