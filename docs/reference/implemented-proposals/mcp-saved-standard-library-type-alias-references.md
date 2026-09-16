---
role: implementation-record
update-when: The MCP references tool schema, saved standard-library type-alias navigation boundary, embedded standard-library type aliases, or executable MCP standard-library type-alias reference evidence changes.
---

# MCP Saved Standard-Library Type-Alias References

The completed slice exposes selected-project references to public type aliases
from exported embedded standard-library modules through the existing MCP
`references` tool. Current behavior is specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md).

Completion evidence:

- The `references-standard-library-type-alias` executable MCP specification
  case checks that the shipped `std::prelude` `ByteCount` public type alias
  returns workspace `file:` result locations and project-wide scope for bare
  prelude and `prelude::`-qualified type occurrences over stdio.
- `veln-language-service` tests check standard-library type-alias identity,
  prelude lookup, import-alias-qualified references, constructor-qualifier
  reference collection, invalid targets, and unsupported alias-chain success
  with empty references while keeping standard-library type-alias `definition`
  unsupported.
- `veln-mcp` server tests check saved-project scope, workspace-only `file:`
  locations, unsupported standard-library type-alias selections, and
  `snapshot_changed` retry exhaustion without success-only fields or
  package-resource state mutation for a standard-library type-alias selection.

Direct-dependency public type-alias references are recorded separately by
[MCP Saved Direct-Dependency Type-Alias References](mcp-saved-dependency-type-alias-references.md).
Schema-alias references, alias-chain traversal, declaration inclusion,
package-source reference locations, recovery or casing-neutral selection,
transitive-dependency references, pagination, rename behavior, new shipped
standard-library aliases, MCP schema expansion, and standard-library
type-alias `definition` support remain outside this standard-library
type-alias `references` result.
