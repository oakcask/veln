---
name: agent-safe-local-runs
description: Use when running broad tests, stress cases, generated-input tests, compiler analysis commands, code-metrics checks, fuzz-like reproducers, or other local commands that may hang or consume large memory. Ensures Codex treats timeouts, kills, and allocation failures as signals of possible leaks, unbounded growth, or nontermination that need investigation.
---

# Agent Safe Local Runs

## Goal

Make runaway local commands observable enough for Codex to notice and
investigate possible memory leaks, unbounded growth, or nontermination. The
guarded entrypoints are a means to produce bounded failure signals, not the
end goal.

## Workflow

1. Use `bash scripts/agent-run <command> [args...]` for any local command that
   may hang, scan large generated inputs, fan out across the workspace, or use
   memory proportional to input size.
2. Use `bash scripts/agent-test [cargo test args...]` for Rust tests. With no
   arguments it runs the workspace test command through the guarded runner.
3. Keep narrow, low-risk commands direct when they are ordinary file reads,
   searches, formatting checks, or single small test binaries.
4. If a guarded command fails with exit code `124`, treat it as a timeout
   signal and inspect the likely slow or looping path before increasing limits.
5. If a guarded command is killed or reports allocation failure, reduce input
   size or parallelism only enough to isolate the source. First consider
   leaks, unbounded collections, repeated whole-input scans, runaway recursion,
   and accidental exponential behavior. Raise the memory limit only when the
   larger working set is expected and relevant to the task.

## Entry Points

- `bash scripts/agent-run <command> [args...]`: wraps a command with wall-clock
  and memory limits.
- `bash scripts/agent-test [cargo test args...]`: wraps `cargo test`; defaults
  to the locked workspace test command when no arguments are provided.

## Limits

- Override wall-clock time with `VELN_AGENT_TIMEOUT`.
- Override memory in megabytes with `VELN_AGENT_MEMORY_MB`.
- Override default Rust build parallelism for `agent-test` with
  `VELN_AGENT_CARGO_JOBS`.

Prefer lowering parallelism before raising memory for broad Rust builds and
tests so the failure remains diagnosable. The guarded runner may use a systemd
scope when available; otherwise it falls back to shell resource limits.
