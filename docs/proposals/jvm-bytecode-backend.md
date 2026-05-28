# JVM Bytecode Backend

Status: implemented

This page routes the implemented JVM bytecode backend proposal. Use it to find
history, completion evidence, and the Java source cleanup result. Use
the specification pages for current command and execution behavior.

## Read First

- Current `run` and `test` behavior:
  [../specification/execution.md](../specification/execution.md) and
  [../specification/commands.md](../specification/commands.md).
- Java source backend cleanup result:
  [../reviews/jvm-bytecode-backend-completion.md](../reviews/jvm-bytecode-backend-completion.md).
- CLI fixture organization:
  [../reference/toolchain-test-harness.md](../reference/toolchain-test-harness.md).

## Outcome

The selected target changed the JVM backend route from `typed IR -> Java source
-> javac -> class files` to `typed IR -> class files`. The ordinary command
path now executes through the host JVM without requiring `javac`; the
specification pages above own those current behavior facts.

The old Java source backend API, internal source generator, and
source-generation tests have been removed. That cleanup result is review
evidence, not current language behavior.

## Read When

- Why the proposal existed:
  [jvm-bytecode-backend-full.md#problem](jvm-bytecode-backend-full.md#problem)
  through
  [jvm-bytecode-backend-full.md#non-goals](jvm-bytecode-backend-full.md#non-goals).
- Migration status:
  [jvm-bytecode-backend-full.md#implementation-status](jvm-bytecode-backend-full.md#implementation-status).
- Harness and structural-test plan from the original proposal:
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
- Proposal promotion mechanics:
  [implementation-route.md](implementation-route.md).

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
- Use the completion review before opening the full proposal when the task is
  cleanup evidence rather than original-gate auditing.
- Do not add Java interop, stable JVM ABI, public class names, or bytecode
  layout guarantees through this proposal.
- Do not promote unrelated JVM behavior through this proposal.
