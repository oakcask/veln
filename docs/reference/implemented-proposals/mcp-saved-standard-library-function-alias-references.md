---
role: implementation-record
update-when: The MCP references tool schema, saved standard-library function-alias navigation boundary, shipped standard-library function aliases, or executable MCP standard-library alias-reference cases change.
---

# MCP Saved Standard-Library Function-Alias References

The completed slice exposes selected-project references to public function
aliases from exported embedded standard-library modules through the existing
MCP `references` tool. Current behavior is specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md).

Completion evidence:

- The `references-standard-library-function-alias` executable MCP
  specification case checks that the shipped `std::prelude`
  `byte_chunk_len` and `byte_view_len` aliases return workspace `file:`
  result locations and project-wide scope for bare calls, bare function-value
  occurrences, and `prelude::`-qualified calls over stdio.
- `veln-language-service` tests check explicit standard-library module alias
  references, implicit prelude alias references, qualified and bare
  function-value occurrences, alias and target identity separation,
  lexical-shadow exclusion, unsupported alias-chain success with empty
  references, and preservation of direct-dependency public function-alias
  exclusion.
- `veln-mcp` server tests check the saved-project adapter boundary for
  standard-library public function-alias references, unsupported alias-chain
  success with empty references, and `snapshot_changed` retry exhaustion
  without success-only fields or package-resource state mutation for a
  standard-library alias selection.

Out-of-scope direct-dependency public function-alias references, type-alias or
schema-alias references, alias-chain traversal, pagination, package-source
reference locations, recovery-symbol selection, rename behavior, and MCP
schema expansion remain outside the implemented `references` result.
