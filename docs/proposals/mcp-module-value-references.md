---
role: proposal
update-when: The planned MCP references projection for module immutable values, its supported identities, declaration inclusion, pagination, or acceptance evidence changes.
---

# MCP Module Immutable-Value References

Expose the existing package-aware module immutable-value reference set through
the MCP `references` tool. The shared language service and LSP already support
this identity. MCP definition and rename also project it, but MCP references
currently returns an empty successful result.

This proposal is ready for implementation. It changes one protocol adapter
capability. It does not change Veln syntax, module-value admission, reference
discovery, dependency capture, or the general MCP references protocol.

## Scope

An initial MCP `references` request selects a valid module immutable value at
one of these workspace occurrences:

- the declaration;
- a direct reference in the declaring module;
- the member leaf of a qualified workspace reference;
- the member leaf of an explicit-import-alias reference;
- an eligible reference to a public value in a retained direct dependency.

The request uses the package, module, and declaration-name identity already
defined by
[Editor Support](../specification/editor-support.md#module-immutable-value-navigation).
Equal initializer data, equal source ranges, and equal names in different
package or module owners do not join identities.

Standard-library module values remain unsupported. Invalid declarations,
unresolved occurrences, references to a different package owner, and source
positions that select no supported identity retain the general MCP empty-result
or failure behavior that applies to the same input condition today.

## Observable Contract

For a supported selection, `references` returns the selected identity's
workspace reference locations. The returned ranges cover only the module-value
name. Direct, qualified, and alias-qualified occurrences that resolve to the
identity are included. Qualifier tokens, the import declaration, same-spelled
functions, lexical bindings, and module values with another owner are excluded.

`include_declaration: false` omits the declaration. For a workspace-owned
identity, `include_declaration: true` adds its workspace declaration exactly
once. For a retained direct-dependency identity, it adds the immutable
`veln-pkg:` declaration exactly once. The package declaration is not converted
to a workspace location when a workspace file has the same source path and
range.

The existing MCP references rules remain authoritative for one-based
Unicode-scalar ranges, URI serialization, duplicate removal, ordering, page
size, continuation cursors, and captured-snapshot lifetime. Adding this symbol
class must not create an alternate ordering or cursor path.

The following state rules apply:

| Event | Result | Preserved state |
| --- | --- | --- |
| Supported initial request | First ordered page and an optional cursor | Saved workspace, published resources, and prior tool results are unchanged. |
| Valid continuation | Next page from the captured reference set | Later workspace or resource changes do not rewrite that cursor's snapshot. |
| Unsupported module-value selection | Ordinary successful empty references result | Existing cursors remain usable and no new cursor is created. |
| Invalid initial input or capture failure | Existing MCP failure for that condition | No partial references or cursor are published; saved workspace, resources, prior results, and existing cursors are unchanged. |
| Invalid, expired, or exhausted continuation | Existing cursor failure for that condition | No workspace, resource, or unrelated cursor state changes. |

## Acceptance Model

All evidence in this section is planned. The current empty-result fixtures are
baseline evidence, not proof of the proposed behavior.

| Case | Input | Required observation | Planned evidence |
| --- | --- | --- | --- |
| M1: workspace references | Select one workspace value from its declaration and from direct, qualified, and alias-qualified occurrences. | Every selection returns the same workspace reference set. Qualifiers and same-spelled non-reference tokens are absent. | Extend the checked `module-value-references` MCP transcript and focused MCP adapter tests. |
| M2: declaration option | Repeat M1 with `include_declaration` false and true. | False omits the declaration. True includes it once without duplicating an occurrence. | The same transcript with exact location assertions and a focused declaration-inclusion test. |
| M3: direct dependency | Select a retained dependency value through direct, qualified, and alias-qualified workspace occurrences. | Every selection returns only that dependency identity's workspace references; declaration inclusion adds its `veln-pkg:` declaration. | Extend the dependency reference transcript and focused dependency tests. |
| M4: owner isolation | Provide equal module/value names in two dependency packages and a colliding workspace path and range. | Each selection retains its package owner. No dependency declaration or occurrence is rewritten as the colliding workspace identity. | Shared-language-service identity tests plus focused MCP projection tests. |
| M5: exclusions | Select a same-spelled function or lexical binding, an invalid or unresolved module-value occurrence, and any available standard-library module value. | Existing supported classes retain their own results. Unsupported selections do not acquire the proposed module-value identity and return their existing empty or failure outcome. | Focused MCP negative tests; retain shared semantic and language-service admission coverage. |
| M6: ordering and pagination | Request a reference set larger than one page with page size one, both with and without the declaration. | Locations use the general URI/range sort. Each continuation returns the next captured location exactly once and the final page omits `next_cursor`. | Checked MCP transcript or focused server pagination test with independently asserted locations. |
| M7: failure preservation | Keep a live references cursor, then issue an invalid module-value request and a request whose saved capture fails. | Both failures preserve the live cursor, saved workspace, published resources, and prior successful results. | Focused server state tests using existing capture-failure and cursor support. |
| M8: adapter agreement | Project the same workspace module-value identity through LSP and MCP references. | After coordinate conversion and declaration-option alignment, both adapters expose the shared reference set rather than separate symbol discovery. | Add a module-value row to the saved-navigation cross-adapter conformance coverage. |

The executable transcript cases belong under `examples/specification/mcp/`.
Adapter and state-transition details belong in focused `veln-mcp` tests.
Shared identity tests remain in `veln-language-service`; do not duplicate its
complete reference-discovery matrix in the adapter.

## Related Work And Design Choice

The current
[module immutable-value navigation contract](../specification/editor-support.md#module-immutable-value-navigation)
defines the package-aware identity, reference membership, exclusions, and
workspace/package location boundary consumed by both adapters. This proposal
adopts that result rather than defining an MCP-specific symbol identity.

[Veln PR #1917](https://github.com/oakcask/veln/pull/1917) added the shared
module-value navigation and the LSP, MCP definition, and MCP rename
projections. Its implementation deliberately left MCP references unsupported,
so it establishes the existing adapter boundary rather than evidence that this
proposal is already implemented. The current
[MCP specification](../specification/mcp.md#saved-workspace-navigation) and checked empty
reference transcripts confirm that boundary on the default branch.

The existing MCP references adapter already owns declaration inclusion,
package-location serialization, ordering, pagination, cursor lifetime, and
failure-state preservation for other supported identities. Reusing that path
keeps module values consistent with the protocol. A separate module-value
pagination or package-location implementation would duplicate those contracts
and could diverge from other symbol classes.

Deferring this slice until the proposed versioned conformance gate exists is
unnecessary. The checked MCP transcripts, focused server tests, shared
language-service tests, and existing cross-adapter conformance harness provide
independent evidence for this bounded capability. The future gate can catalog
the evidence after it exists; it is not an implementation prerequisite.

## Non-Goals

- Do not change module immutable-value syntax, typing, initializer evaluation,
  visibility, or semantic admission.
- Do not add standard-library module-value navigation or transitive-dependency
  capture.
- Do not change definition, prepare-rename, rename, hover, completion, or
  signature help.
- Do not add a new MCP field, coordinate system, sort order, cursor type, or
  package URI format.
- Do not broaden references for another symbol class or for recovery identities.

## Implementation And Specification Route

Implementation should admit the existing shared `ModuleValue` result at the
MCP workspace and retained direct-dependency reference boundaries. It should
continue to reject a standard-library-owned module value. No new module-value
index or adapter-local reference scan is required.

Add executable evidence before updating current-behavior prose. When the
evidence passes, update the saved-navigation and supported-reference sections
of `mcp.md`, including the module-value fixture descriptions. Remove the
implemented slice from the agent-language-services inventory, then remove this
proposal and its Ready catalog entry once the current specification and
checked evidence own the behavior.
