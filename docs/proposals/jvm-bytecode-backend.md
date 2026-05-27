# JVM Bytecode Backend

Status: implemented

This page routes the implemented proposal that makes the JVM backend emit JVM
class files directly. Current implemented behavior remains under
[../specification/execution.md](../specification/execution.md).

## Read First

- Implementation status and remaining proposal work:
  [jvm-bytecode-backend-full.md#implementation-status](jvm-bytecode-backend-full.md#implementation-status).
- Completion criteria:
  [jvm-bytecode-backend-full.md#completion-criteria](jvm-bytecode-backend-full.md#completion-criteria).
- Current completion review:
  [../reviews/jvm-bytecode-backend-completion.md](../reviews/jvm-bytecode-backend-completion.md).
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

## Current Handoff

The ordinary `run` and `test` path now lowers typed IR to JVM classfile
artifacts and no longer writes generated Java source or invokes a Java source
compiler during ordinary execution. The implemented command-visible behavior is
promoted in [../specification/execution.md](../specification/execution.md) and
[../specification/commands.md](../specification/commands.md).

The Java source backend API remains only as a migration baseline for backend
tests. Current review evidence and follow-up cleanup live in
[../reviews/jvm-bytecode-backend-completion.md](../reviews/jvm-bytecode-backend-completion.md).

## Detail Routes

- Problem, decision, and explicit non-goals:
  [jvm-bytecode-backend-full.md#problem](jvm-bytecode-backend-full.md#problem)
  through
  [jvm-bytecode-backend-full.md#non-goals](jvm-bytecode-backend-full.md#non-goals).
- Runtime parity harness, fixture scope, and structural checks:
  [runtime behavior harness](jvm-bytecode-backend-full.md#runtime-behavior-harness)
  through
  [bytecode verification coverage](jvm-bytecode-backend-full.md#bytecode-verification-coverage).
- CI and setup behavior:
  [CI strategy](jvm-bytecode-backend-full.md#ci-strategy)
  and
  [cache and setup behavior](jvm-bytecode-backend-full.md#cache-and-setup-behavior).
- Completion criteria, implementation notes, and promotion cleanup:
  [completion criteria](jvm-bytecode-backend-full.md#completion-criteria),
  [implementation notes](jvm-bytecode-backend-full.md#implementation-notes),
  and [promotion route](jvm-bytecode-backend-full.md#promotion-route).
- Current handoff review for why the proposal remains open:
  [../reviews/jvm-bytecode-backend-completion.md](../reviews/jvm-bytecode-backend-completion.md).

## Boundary

This proposal does not change Veln source semantics, typed IR semantics,
runtime value freezing, stdio ordering, contract behavior, test event shape,
task behavior, channel behavior, or the rule that JVM names and layouts are
backend details.

## Skip Unless Needed

- Do not open [jvm-bytecode-backend-full.md](jvm-bytecode-backend-full.md)
  before choosing one detail route above.
- Use the specification pages, not this proposal page, for current JVM backend
  behavior.
- Do not add Java interop, stable JVM ABI, public class names, or bytecode
  layout guarantees through this proposal.
- Do not promote unrelated JVM behavior through this proposal.
