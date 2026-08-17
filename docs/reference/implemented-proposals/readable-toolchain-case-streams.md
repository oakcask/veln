---
role: implementation-record
authority: supporting
update-when: The readable toolchain case stream completion evidence, structured JSON-RPC LSP fixture evidence, decoded assertion baseline, or current harness reference route changes.
---

# Readable Toolchain Case Streams

## Current Behavior Route

Use [../toolchain-test-harness.md](../toolchain-test-harness.md) for the
current harness contract. Use
[../../specification/editor-support.md](../../specification/editor-support.md)
for the editor-facing LSP examples that exercise the migrated cases.

This record is historical completion evidence. It is not the source for
current harness behavior.

## Completed Scope

The readable case stream work moved review-sensitive protocol fixture text out
of manifest escape sequences and into structured or file-backed forms:

- `stdin_jsonrpc_file` carries ordered JSON-RPC requests and notifications for
  LSP cases whose behavior is decoded message structure.
- `$case_text` directives let structured requests include exact case-relative
  document text without manual protocol framing.
- `[[lsp_assert]]` selectors check decoded responses and notifications by id,
  method occurrence, and JSON Pointer.
- Raw stream fixtures remain available for cases whose observable behavior is
  protocol bytes, framing failures, or an as-yet-unmigrated representation.

Representative LSP cases now use structured request fixtures and decoded
assertions for publish diagnostics, semantic tokens, and semantic tokens after
an unsaved document change. Later navigation cases can still use raw
`stdin_file` sidecars when the protocol stream itself is part of the fixture;
the semantic baseline records those sidecar bytes alongside the case stream
fragments so the remaining raw coverage stays reviewable.

## Completion Evidence

The implemented harness contract is covered by the `decoded_lsp_*`,
`raw_stdout_and_decoded_lsp_*`, `repeated_run_failures_*`, and
`manifest_jsonrpc_*` tests in `toolchain_harness.rs`.

The migrated executable examples are:

- `examples/specification/lsp/publish-diagnostics/`
- `examples/specification/lsp/semantic-tokens/`
- `examples/specification/lsp/semantic-tokens-unsaved-change/`

The checked semantic baseline records each migrated selector, path, operation,
operand, sidecar, and structured request fixture so later migrations can be
reviewed against the same inventory.

## Closure

The active proposal page was removed from `docs/proposals/` after the
representative LSP migration moved into executable examples and the current
harness and editor-support references.
