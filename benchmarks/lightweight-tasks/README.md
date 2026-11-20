# JVM lightweight-task adoption benchmark

`result.json` is the checked controlled comparison between the current JVM
virtual-thread worker and a platform-thread baseline built from the same runtime
source. The baseline preparation changes only the worker constructor. The
artifact records the two source digests, fixed workload identity, raw samples,
summaries, diagnostics, and every adoption verdict.

Run the controlled collection from the repository root with:

```sh
VELN_AGENT_TIMEOUT=900 VELN_AGENT_MEMORY_MB=16384 \
  bash scripts/agent-run bash scripts/benchmark-lightweight-tasks
```

The benchmark runner enforces the 4 GiB Java process RSS ceiling itself. The
larger outer address-space guard is necessary because the fallback guard limits
reserved virtual address space when a user systemd manager is unavailable.

Validate the checked artifact without collecting new timing samples with:

```sh
bash scripts/benchmark-lightweight-tasks --check \
  benchmarks/lightweight-tasks/result.json
```
