---
role: implementation-record
update-when: The MCP references tool schema, saved direct-dependency type-alias navigation boundary, or executable MCP dependency type-alias-reference cases change.
---

# MCP Saved Direct-Dependency Type-Alias References

The completed slice exposes selected-project references to public type aliases
from exported direct-dependency modules through the existing MCP `references`
tool. Current behavior is specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md).

Completion evidence:

- The `references-dependency-type-alias` executable MCP specification case
  checks visible direct-dependency public type-alias references through type
  annotations, type arguments, return types, type-alias right-hand sides,
  constructor qualifiers, written module paths, and import-alias-qualified
  paths. It also checks project-wide scope, workspace `file:` result
  locations, alias and target-type identity separation, package collisions,
  lexical noise exclusion, unsupported alias-chain selection, unresolved,
  wrong-kind, and invalid-casing alias targets, descendant project isolation,
  and dependency source resource admission.
- `veln-language-service` tests check direct-dependency public type-alias
  selection and reference collection for type annotations, type arguments,
  return types, type-alias right-hand sides, constructor qualifiers, canonical
  project-wide results, alias and target-type identity separation, package
  collisions, retained non-exported implementation source targets, invalid
  targets, invalid-casing alias declarations, and unsupported alias chains.
- `veln-mcp` server tests check the saved-project adapter boundary for
  direct-dependency public type-alias references, target-type separation,
  anonymous source, descendant project, and outside-selected-project isolation,
  invalid target success with empty references, and `snapshot_changed` retry
  exhaustion without success-only fields or package-resource state mutation
  for a direct-dependency type-alias selection.

Out-of-scope standard-library type-alias references, public schema-alias
references, alias-chain traversal, pagination, declaration inclusion,
package-source reference locations, recovery-symbol selection, transitive
dependency references, casing-neutral selection, rename behavior, and MCP
schema expansion remain outside the implemented `references` result.
