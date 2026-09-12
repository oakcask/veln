---
role: proposal
update-when: The embedded standard-library export surface gains a public function alias, or the saved MCP standard-library alias-reference boundary or planned evidence changes.
---

# MCP Saved Standard-Library Function-Alias References

## Summary

Extend the existing MCP `references` tool to find selected-project uses of a
public function alias from an exported embedded standard-library module. Keep
the current unpaginated result schema and reference-only workspace location
boundary.

## Readiness

This proposal is ready because the embedded standard library exports the
public `std::prelude` byte-length function aliases recorded by
[Standard-Library Byte Length Aliases](../reference/implemented-proposals/standard-library-byte-length-aliases.md).
Those aliases provide a shipped success path for saved MCP standard-library
alias references without using a test-only replacement package.

## Scope

After the prerequisite exists, selection must follow the saved-project
explicit-import, implicit-prelude, visibility, and shadowing rules used by
definition lookup. Results must include qualified calls, qualified
function-value occurrences, and accepted bare implicit-prelude forms that
resolve to the selected alias.

Alias identity remains distinct from the target function. Alias declarations,
alias targets, direct target-function uses, package source bodies, collisions,
unsupported alias chains, recovery records, and sources outside the selected
project remain excluded. This proposal does not add a standard-library alias,
public type-alias or schema-alias references, alias-chain traversal,
pagination, package-source locations, recovery selection, or rename behavior.

## Acceptance Model

| Case | Expected result | Planned evidence |
| --- | --- | --- |
| Select the prerequisite exported standard-library public function alias through an explicit import. | Return qualified calls and qualified function-value occurrences that resolve to the alias. | Language-service standard-library alias cases and an executable MCP specification case using the shipped alias. |
| Select a prerequisite implicit-prelude public function alias through bare and `prelude::` forms. | Return accepted bare and qualified occurrences while excluding lexical shadows. | Language-service prelude alias table and the executable MCP specification case when the prerequisite alias is in the prelude. |
| Select the alias target, a collision, an unsupported alias chain, or a source outside the selected project. | Preserve alias identity, unsupported-selection success, and selected-project isolation. | Language-service identity and boundary tables plus MCP adapter cases. |
| Change selected-project or embedded package inputs until retry exhaustion. | Return `snapshot_changed` without locations or scope and preserve package-resource state. | Saved-reference capture and state-preservation cases using the shipped alias. |

## Completion

This proposal is complete when its prerequisite is current implemented
standard-library behavior, every applicable acceptance row has checked
evidence, the shipped alias success path appears in an executable MCP
specification case, and the current MCP specification states the implemented
boundary. The implementation must then remove this page from the active
proposal catalog.
