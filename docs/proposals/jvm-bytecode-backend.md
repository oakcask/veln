# JVM Bytecode Backend

Status: open-proposal

This page tracks the proposal to make the JVM backend emit JVM class files
directly instead of treating Java source generation as the long-term lowering
route. Current implemented behavior remains under
[../reference/language/execution.md](../reference/language/execution.md).

## Read First

- Current execution and JVM backend behavior:
  [../reference/language/execution.md](../reference/language/execution.md).
- Current command setup behavior:
  [../reference/language/commands.md](../reference/language/commands.md).
- Current CLI fixture harness:
  [../reference/toolchain-test-harness.md](../reference/toolchain-test-harness.md).
- Full proposal, test strategy, and CI plan:
  [jvm-bytecode-backend-full.md](jvm-bytecode-backend-full.md).

## Decision

Change the JVM backend implementation route from `typed IR -> Java source ->
javac -> class files` to `typed IR -> class files`.

The JVM remains the reference execution target for `run` and `test`. The
selected entry still executes through the host JVM, and missing `java` remains
a runner setup failure. Direct classfile emission means the bytecode backend
does not require `javac` for ordinary `run` or `test` execution.

This proposal does not change Veln source semantics, typed IR semantics,
runtime value freezing, stdio ordering, contract behavior, test event shape,
or the rule that JVM names and layouts are backend details.

## Test Strategy

The bytecode backend must be introduced with runtime behavior coverage, not
only backend unit tests. During migration, a backend-matrix harness should run
selected executable fixtures through both the Java source backend and the
bytecode backend, then compare observable command behavior.

The parity comparison boundary is exit status, stdout, stderr, structured JSON
records, test events, and runtime contract failures. Runner setup behavior,
including missing `java` and the bytecode backend's lack of an ordinary `javac`
requirement, should be covered by setup-specific tests instead of parity
comparison. The harness must not compare generated Java source, classfile bytes,
bytecode instruction sequences, constant-pool indexes, class names, local
variable slots, or helper layout.

## CI Strategy

Required pull request checks should include one pinned-JDK JVM backend job that
runs bytecode runtime fixtures and bytecode structural checks. Broader
operating-system and JDK-line coverage can run as scheduled or optional matrix
jobs once the required path is stable.

`javap -verbose` should be used for structural classfile smoke tests, while
loading and executing generated classes with `java` remains the authoritative
JVM verifier boundary.

## Skip Unless Needed

- Do not use this page as current JVM backend behavior until the reference is
  updated after implementation.
- Do not add Java interop, stable JVM ABI, public class names, or bytecode
  layout guarantees through this proposal.
- Do not promote this proposal to implementation work unless the target queue
  selects it as an accepted target.
