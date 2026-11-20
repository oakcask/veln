# JVM lightweight-task adoption benchmark

`result.json` is the checked controlled comparison between the current JVM
platform-thread worker and a virtual-thread candidate built from the same runtime
source. Candidate preparation changes only the worker constructor. The
artifact records the two source digests, fixed workload identity, raw samples,
summaries, diagnostics, and every adoption verdict.

The loopback resource gate requires Linux process-descriptor measurement to
succeed before and after the workload. The harness closes every stream and the
listener before the second measurement, and every measured run must return to
zero descriptor growth. Selector-operation count is a diagnostic rather than
an observed resource-closure signal.

The runner observes RSS and OS-thread peaks externally while each parked-task
population is being created and while that population is fully parked. The
1,024-task observations own the common-population gates; the 256-task and
16,384-task observations own the candidate OS-thread growth gate.

Run the controlled collection from the repository root with:

```sh
VELN_AGENT_TIMEOUT=900 VELN_AGENT_MEMORY_MB=32768 \
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
virtual-thread candidate, and verifies both source digests. It also recomputes
all summaries and adoption gates from the stored raw samples before accepting
the artifact. A checked artifact with a reproduced failed adoption verdict
returns a nonzero status and names the failed gates; this preserves measured
blocking evidence without presenting it as adoption success.
