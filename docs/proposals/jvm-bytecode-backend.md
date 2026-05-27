# JVM Bytecode Backend

Status: proposed

This page routes the selected proposal to make the JVM backend emit JVM class
files directly. Current implemented behavior remains under
[../specification/execution.md](../specification/execution.md); do not use this
proposal as current behavior until implementation is promoted there.

## Read First

- Current execution and JVM backend behavior:
  [../specification/execution.md](../specification/execution.md).
- Current command setup behavior:
  [../specification/commands.md](../specification/commands.md).
- Current CLI fixture harness:
  [../reference/toolchain-test-harness.md](../reference/toolchain-test-harness.md).
- Proposal promotion route:
  [implementation-route.md](implementation-route.md).

## Target

Change the JVM backend implementation route from `typed IR -> Java source ->
javac -> class files` to `typed IR -> class files`.

The JVM remains the reference execution target for `run` and `test`. The
selected entry still executes through the host JVM, and missing `java` remains
a runner setup failure. Direct classfile emission means the bytecode backend
does not require `javac` for ordinary `run` or `test` execution.

## Detail Routes

- Problem and decision:
  [jvm-bytecode-backend-full.md#problem](jvm-bytecode-backend-full.md#problem)
  and
  [jvm-bytecode-backend-full.md#decision](jvm-bytecode-backend-full.md#decision).
- Runtime parity harness and fixture scope:
  [runtime behavior harness](jvm-bytecode-backend-full.md#runtime-behavior-harness)
  and
  [fixture scope](jvm-bytecode-backend-full.md#fixture-scope).
- Bytecode verification and CI:
  [bytecode verification coverage](jvm-bytecode-backend-full.md#bytecode-verification-coverage)
  and
  [CI strategy](jvm-bytecode-backend-full.md#ci-strategy).
- Cache, setup, acceptance criteria, and working answers:
  [cache and setup behavior](jvm-bytecode-backend-full.md#cache-and-setup-behavior),
  [acceptance criteria](jvm-bytecode-backend-full.md#acceptance-criteria),
  and [working answers](jvm-bytecode-backend-full.md#working-answers).

## Boundary

This proposal does not change Veln source semantics, typed IR semantics,
runtime value freezing, stdio ordering, contract behavior, test event shape,
task behavior, channel behavior, or the rule that JVM names and layouts are
backend details.

## Skip Unless Needed

- Do not open [jvm-bytecode-backend-full.md](jvm-bytecode-backend-full.md)
  before choosing one detail route above.
- Do not use this page as current JVM backend behavior until the specification
  is updated after implementation.
- Do not add Java interop, stable JVM ABI, public class names, or bytecode
  layout guarantees through this proposal.
- Do not promote unrelated JVM behavior through this proposal.
