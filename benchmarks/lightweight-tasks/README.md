# JVM lightweight-task adoption benchmark

`result.json` is the checked controlled comparison between the current JVM
virtual-thread worker and a platform-thread baseline built from the same runtime
source. Baseline preparation changes only the worker constructor. The
artifact records the two source digests, fixed workload identity, raw samples,
summaries, diagnostics, and every adoption verdict.

Each sample uses separate parked-task and common-cost JVM phases with identical
runtime identity and JVM arguments. Each phase keeps the fixed per-JVM timeout.
The runner combines the population observations and cost results before it
evaluates the sample.

The loopback resource gate requires Linux process-descriptor measurement to
succeed before and after the workload. The harness closes every stream and the
listener before the second measurement, and every measured run must return to
zero descriptor growth. Selector-operation count is a diagnostic rather than
an observed resource-closure signal.

The runner observes RSS and OS-thread peaks externally while each parked-task
population is being created and while that population is fully parked. The
1,024-task observations own the common-population gates; the 256-task and
16,384-task observations own the candidate OS-thread growth gate.

The channel workloads also record non-gating p95 diagnostics for the
producer-to-relay handoff, relay processing, relay-to-consumer handoff, and a
paired `park`/`unpark` control. These measurements separate worker scheduling
from channel monitor reacquisition without changing the governed latency gates.
To collect one discarded run of each variant and
print only those channel diagnostics, run:

```sh
VELN_AGENT_TIMEOUT=900 VELN_AGENT_MEMORY_MB=32768 \
  bash scripts/agent-run bash scripts/benchmark-lightweight-tasks --diagnostic-only
```

Run the controlled collection from the repository root with:

```sh
VELN_AGENT_TIMEOUT=3000 VELN_AGENT_MEMORY_MB=32768 \
  bash scripts/agent-run bash scripts/benchmark-lightweight-tasks
```

The benchmark runner enforces the 4 GiB Java process RSS ceiling itself. The
larger outer address-space guard is necessary because the fallback guard limits
reserved virtual address space, including platform-thread stacks, when a user
systemd manager is unavailable.

Validate the checked artifact without collecting new timing samples with:

```sh
bash scripts/benchmark-lightweight-tasks --check \
  benchmarks/lightweight-tasks/result.json
```

Validation rebuilds the current generated JVM runtime, derives the controlled
platform-thread baseline, and verifies both source digests. It also recomputes
all summaries and adoption gates from the stored raw samples before accepting
the artifact. JVM CI runs this validation. A checked artifact with a reproduced
failed adoption verdict returns a nonzero status and names the failed gates.
