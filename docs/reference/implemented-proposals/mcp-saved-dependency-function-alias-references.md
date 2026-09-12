---
role: implementation-record
update-when: The MCP references tool schema, saved direct-dependency function-alias navigation boundary, or executable MCP dependency alias-reference cases change.
---

# MCP Saved Direct-Dependency Function-Alias References

The completed slice exposes selected-project references to public function
aliases from exported direct-dependency modules through the existing MCP
`references` tool. Current behavior is specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md).

Completion evidence:

- The `references-dependency-function-alias` executable MCP specification case
  checks visible direct-dependency public function-alias references through
  qualified calls, import-alias-qualified calls, and qualified function-value
  occurrences. It also checks project-wide scope, workspace `file:` result
  locations, alias and target-function identity separation, package and
  workspace collisions, field exclusion, unsupported alias-chain selection,
  unresolved, wrong-kind, and invalid-casing alias targets, descendant
  project isolation, and dependency source resource admission.
- `veln-language-service` tests check direct-dependency public function-alias
  selection and reference collection for qualified calls, import-alias-qualified
  calls, qualified function-value occurrences, canonical project-wide results,
  alias and target-function identity separation, package collisions, workspace
  function collisions, field exclusion, retained non-exported implementation
  source targets, invalid targets, invalid-casing alias declarations, and
  unsupported alias chains whose targets are in the same, another, or
  non-exported captured dependency module.
- `veln-mcp` server tests check the saved-project adapter boundary for
  direct-dependency public function-alias references, target-function
  separation, anonymous source, descendant project, and outside-selected-project
  isolation, unsupported alias-chain success with empty references, invalid
  target success with empty references, and `snapshot_changed` retry exhaustion
  without success-only fields or package-resource state mutation for a
  direct-dependency alias selection.

Out-of-scope public type-alias or public schema-alias references, alias-chain
traversal, pagination, package-source reference locations, recovery-symbol
selection, transitive-dependency references, casing-neutral selection, rename
behavior, and MCP schema expansion remain outside the implemented
`references` result.
