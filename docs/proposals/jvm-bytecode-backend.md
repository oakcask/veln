# JVM Bytecode Backend

Status: implemented

This page routes the implemented JVM bytecode backend proposal. Use it for
history, completion evidence, and cleanup. Use the specification pages for
current command and execution behavior.

## Read First

- Current execution and JVM backend behavior:
  [../specification/execution.md](../specification/execution.md).
- Current command setup behavior:
  [../specification/commands.md](../specification/commands.md).
- Current CLI fixture harness:
  [../reference/toolchain-test-harness.md](../reference/toolchain-test-harness.md).
- Completion review and follow-up cleanup:
  [../reviews/jvm-bytecode-backend-completion.md](../reviews/jvm-bytecode-backend-completion.md).
- Original gates:
  [jvm-bytecode-backend-full.md#completion-criteria](jvm-bytecode-backend-full.md#completion-criteria).
- Proposal promotion route:
  [implementation-route.md](implementation-route.md).

## Outcome

The selected target changed the JVM backend route from `typed IR -> Java source
-> javac -> class files` to `typed IR -> class files`.

The JVM remains the reference execution target for `run` and `test`. The
selected entry still executes through the host JVM, and missing `java` remains
a runner setup failure. Direct classfile emission means the bytecode backend
does not require `javac` for ordinary `run` or `test` execution.

## Current Handoff

For current behavior, read
[../specification/execution.md](../specification/execution.md) and
[../specification/commands.md](../specification/commands.md). For test harness
organization, read
[../reference/toolchain-test-harness.md](../reference/toolchain-test-harness.md).
For remaining migration cleanup, read
[../reviews/jvm-bytecode-backend-completion.md](../reviews/jvm-bytecode-backend-completion.md).

## Detail Routes

- Problem, decision, and explicit non-goals:
  [jvm-bytecode-backend-full.md#problem](jvm-bytecode-backend-full.md#problem)
  through
  [jvm-bytecode-backend-full.md#non-goals](jvm-bytecode-backend-full.md#non-goals).
- Historical implementation status:
  [jvm-bytecode-backend-full.md#implementation-status](jvm-bytecode-backend-full.md#implementation-status).
- Runtime parity harness, fixture scope, and structural checks from the
  original proposal:
  [runtime behavior harness](jvm-bytecode-backend-full.md#runtime-behavior-harness)
  through
  [bytecode verification coverage](jvm-bytecode-backend-full.md#bytecode-verification-coverage).
- CI and setup behavior:
  [CI strategy](jvm-bytecode-backend-full.md#ci-strategy)
  and
  [cache and setup behavior](jvm-bytecode-backend-full.md#cache-and-setup-behavior).
- Cache security requirements from the proposal live in
  [cache and setup behavior](jvm-bytecode-backend-full.md#cache-and-setup-behavior):
  cache hits must validate a manifest keyed by a cryptographic digest before
  executing cached classes.
- Completion criteria, implementation notes, and promotion cleanup:
  [completion criteria](jvm-bytecode-backend-full.md#completion-criteria),
  [implementation notes](jvm-bytecode-backend-full.md#implementation-notes),
  and [promotion route](jvm-bytecode-backend-full.md#promotion-route).
- Current handoff review and follow-up cleanup:
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
- Use the completion review before opening the full proposal when the task is
  cleanup rather than original-gate auditing.
- Do not add Java interop, stable JVM ABI, public class names, or bytecode
  layout guarantees through this proposal.
- Do not promote unrelated JVM behavior through this proposal.
