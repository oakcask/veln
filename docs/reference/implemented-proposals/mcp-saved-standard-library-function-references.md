---
role: implementation-record
update-when: The MCP references tool schema, saved standard-library navigation boundary, standard-library function-reference behavior, or executable MCP standard-library-reference cases change.
---

# MCP Saved Standard-Library Function References

The completed slice exposes references to public function declarations from
exported embedded standard-library modules through the existing MCP
`references` tool. Current behavior is specified by
[MCP Workspace Projects, Resources, And Navigation](../../specification/mcp.md).

Completion evidence:

- The `references-standard-library-function` executable MCP specification case
  checks accepted bare implicit prelude calls, qualified calls, qualified
  function-value occurrences, workspace `file:` result locations,
  project-wide scope metadata, and collision exclusion for workspace,
  dependency, field, string, comment, declaration, package-source, and
  import-alias occurrences.
- `veln-language-service` tests check explicit standard-library module
  function references, implicit prelude function references, qualified
  function-value references, package-origin identity, lexical shadowing,
  collision exclusion, private and non-exported visibility, and
  invalid-casing rejection.
- `veln-mcp` server tests check the saved-project adapter boundary for
  standard-library public function references, collision filtering,
  `snapshot_changed` retry exhaustion without success-only fields, and package
  public type-alias selections empty.

Out-of-scope pagination, declaration inclusion, package-source reference
locations, non-function package symbols, recovery symbols, and
transitive-dependency references remain outside the implemented `references`
result. Standard-library public function-alias references are recorded
separately by
[MCP Saved Standard-Library Function-Alias References](mcp-saved-standard-library-function-alias-references.md).
