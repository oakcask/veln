---
role: reference
authority: supporting
update-when: The JVM task worker mechanism, retained wait algorithms, or controlled lightweight-task adoption evidence changes.
---

# JVM Lightweight Task Worker Selection

## Decision

Keep one non-daemon platform thread per JVM task. Do not adopt the evaluated
one-unpooled-virtual-thread-per-task candidate with the retained channel
monitor, polling select, socket selector, and bounded DNS-worker algorithms.

The virtual-thread candidate was evaluated against fixed functional,
allocation, resource, performance, noise, and retained-state gates. The
corrected controlled comparison did not satisfy both channel delivery-latency
gates or the complete noise gate. A failed required gate blocks adoption. It
does not justify weakening the selected budgets or changing a second runtime
algorithm only to obtain a passing comparison.

The implementation and current execution specification therefore continue to
own platform-thread task workers. The rejected implementation and its bounded
comparison remain available in [Veln PR #1985](https://github.com/oakcask/veln/pull/1985)
as historical evidence, not as current behavior or an active proposal.

## Scope

This decision rejects the measured candidate at the selected comparison
boundary. It does not establish that Java virtual threads always have
unacceptable latency. It also does not reject lightweight task execution as a
language goal.

A future proposal needs an independently justified worker or suspension
mechanism and its own complete acceptance model. Reconsider this decision when
the supported JVM changes relevant wait behavior, an independently motivated
wait-algorithm change lands, or another mechanism has evidence that it can
preserve task, cancellation, handler, cleanup, and host-effect contracts while
meeting explicit allocation and runtime-cost gates.
