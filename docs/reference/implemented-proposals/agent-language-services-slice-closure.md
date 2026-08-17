---
role: implementation-record
authority: supporting
update-when: The MCP JSONL assertion contract, agent-language-services lifecycle split, shared-capture evidence rule, or named completion evidence is superseded or invalidated.
---

# Agent Language Services Slice Closure

This record closes the finite lifecycle and executable-evidence gate that
preceded the next agent-language-services slice. Current behavior is defined by
the linked specification, harness reference, and executable cases.

## Completed Boundary

- The active agent-language-services proposal now contains only unresolved
  capabilities and a finite next-slice acceptance model.
- Completed foundations are routed through
  [Agent Language Service Foundations](agent-language-services-foundations.md).
- Repeatable `[[mcp_assert]]` sections select exactly one JSONL response by a
  string or integer ID and apply an RFC 6901 JSON Pointer.
- MCP assertions support complete JSON equality, exact array length, missing
  values, and canonical case-workspace file URIs.
- The harness rejects malformed or non-object JSONL lines, invalid pointers,
  missing or duplicate selected IDs, invalid operation combinations, wrong
  value kinds, and unsafe workspace-file operands.
- Shared stable-capture evidence may be composed with a focused adapter-route
  test and an adapter result test that proves `snapshot_changed` without
  partial success fields.

## Evidence

- [Toolchain Test Harness](../toolchain-test-harness.md) defines the current
  assertion contract.
- `toolchain_harness.rs` contains the accepted and rejected contract matrices,
  stream decoding cases, workspace URI boundary cases, and failure aggregation
  case.
- `examples/specification/mcp/definition-workspace/` proves a dynamic workspace
  URI, indexed array content, cardinality, exact range, and absent response
  fields through response-local assertions.
- The checked toolchain semantic baseline records every MCP assertion selector,
  operation, and operand in that executable case.

The next selectable work is the bounded saved workspace function-reference
slice in [Agent Language Services](../../proposals/agent-language-services.md).
