---
name: effect-handler-maintenance
description: Implement, change, or review effect handlers, host effect boundaries, or deterministic effect test support in Veln.
---

# Effect Handler Maintenance

## Authority and Scope

Read the relevant behavior in `docs/specification/effects.md` and
`docs/specification/execution.md` before changing handler semantics. Use
`docs/reference/toolchain-test-harness.md` when changing fixture support.
The root `AGENTS.md` owns the rule requiring scoped Veln fakes and real host
defaults; this skill covers the implementation and verification workflow.

## Handler Boundaries

Preserve these properties when changing the affected boundary:

- Child tasks inherit their execution's active effect handlers. Independent
  executions receive separate fake state.
- Leaving an injected scope restores the preceding handler after normal
  completion, early error propagation, and runtime exceptions.
- Network resources retain their creating handler provider. Using a resource
  after its creating scope exits must not silently switch it to real host I/O.
- Fake behavior and state belong in separate Veln test support. Genuine
  `process::env` operations and diagnostic output destinations remain separate
  from effect substitution.

When extending a fake, preserve the affected assertions for operation order,
bytes, resource identity, commit state, and cleanup. Choose coverage for the
behavior being changed rather than duplicating implementation details.

## Verification

Run commands from the repository root through the guarded test runner:

- Check ambient selectors with
  `bash scripts/agent-test -p veln-backend-jvm effect_boundary_policy`.
- Check inheritance, scope restoration, and provider ownership with
  `bash scripts/agent-test -p veln-backend-jvm effect_injection` when changing
  handler dispatch, task propagation, or resource lifetime.

Use the affected Veln fixtures for fake behavior and host-boundary tests for
real I/O. An injected fake passing does not establish that an unhandled
operation still performs the specified host effect.
